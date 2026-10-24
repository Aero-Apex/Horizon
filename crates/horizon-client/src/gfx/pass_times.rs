//! `HORIZON_PASS_TIMES=1`: GPU time of every render pass of the shader pack,
//! from timestamp queries on the passes themselves, logged as averages every
//! 120 frames. For finding which pass makes a frame slow; it waits for the GPU
//! each frame, so frame rates read lower while it is on. Off, a pass costs one
//! atomic load.
use std::{
    collections::BTreeMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU32, Ordering},
    },
};

const MAX_PASSES: u32 = 96;
const REPORT_EVERY: u32 = 120;

struct Times {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    period_ns: f32,
    features: wgpu::Features,
    next: AtomicU32,
    names: Mutex<Vec<String>>,
    sums: Mutex<(u32, BTreeMap<String, (f64, u32)>)>,
}

static TIMES: OnceLock<Option<Times>> = OnceLock::new();

/// Starts timing when `HORIZON_PASS_TIMES` is set and the device can.
pub fn init(device: &wgpu::Device, queue: &wgpu::Queue) {
    TIMES.get_or_init(|| {
        if std::env::var_os("HORIZON_PASS_TIMES").is_none()
            || !device.features().contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return None;
        }
        let size = u64::from(MAX_PASSES) * 2 * 8;
        log::info!("Horizon: timing the shader pack's passes (HORIZON_PASS_TIMES)");
        Some(Times {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Pass timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: MAX_PASSES * 2,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Pass timestamp resolve"),
                size,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            read: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Pass timestamp readback"),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            period_ns: queue.get_timestamp_period(),
            features: device.features(),
            next: AtomicU32::new(0),
            names: Mutex::new(Vec::new()),
            sums: Mutex::new((0, BTreeMap::new())),
        })
    });
}

/// The timestamp writes for a pass called `label`, when timing is on.
pub fn writes(label: &str) -> Option<wgpu::RenderPassTimestampWrites<'static>> {
    let times = TIMES.get()?.as_ref()?;
    let slot = times.next.fetch_add(1, Ordering::Relaxed);
    if slot >= MAX_PASSES {
        return None;
    }
    let mut names = times.names.lock().ok()?;
    names.truncate(slot as usize);
    names.push(label.to_owned());
    Some(wgpu::RenderPassTimestampWrites {
        query_set: &times.set,
        beginning_of_pass_write_index: Some(slot * 2),
        end_of_pass_write_index: Some(slot * 2 + 1),
    })
}

/// Starts timing commands that are not a pass (copies): the slot to give
/// [`stamp_end`].
pub fn stamp(encoder: &mut wgpu::CommandEncoder, label: &str) -> Option<u32> {
    let times = TIMES.get()?.as_ref()?;
    if !times
        .features
        .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
    {
        return None;
    }
    let slot = times.next.fetch_add(1, Ordering::Relaxed);
    if slot >= MAX_PASSES {
        return None;
    }
    let mut names = times.names.lock().ok()?;
    names.truncate(slot as usize);
    names.push(label.to_owned());
    encoder.write_timestamp(&times.set, slot * 2);
    Some(slot)
}

/// Ends what [`stamp`] started.
pub fn stamp_end(encoder: &mut wgpu::CommandEncoder, slot: Option<u32>) {
    if let (Some(slot), Some(Some(times))) = (slot, TIMES.get()) {
        encoder.write_timestamp(&times.set, slot * 2 + 1);
    }
}

/// At the end of the frame's commands: copies the stamps out.
pub fn resolve(encoder: &mut wgpu::CommandEncoder) {
    let Some(Some(times)) = TIMES.get() else {
        return;
    };
    let used = times.next.load(Ordering::Relaxed).min(MAX_PASSES);
    if used == 0 {
        return;
    }
    let bytes = u64::from(used) * 2 * 8;
    encoder.resolve_query_set(&times.set, 0..used * 2, &times.resolve, 0);
    encoder.copy_buffer_to_buffer(&times.resolve, 0, &times.read, 0, bytes);
}

/// After the frame is submitted: waits for it and adds its passes up.
pub fn collect(device: &wgpu::Device) {
    let Some(Some(times)) = TIMES.get() else {
        return;
    };
    let used = times.next.swap(0, Ordering::Relaxed).min(MAX_PASSES);
    if used == 0 {
        return;
    }
    let bytes = u64::from(used) * 2 * 8;
    let slice = times.read.slice(..bytes);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    if !matches!(rx.recv(), Ok(Ok(()))) {
        return;
    }
    let stamps: Vec<u64> = {
        let Ok(data) = slice.get_mapped_range() else {
            return;
        };
        data.chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes")))
            .collect()
    };
    times.read.unmap();
    let (Ok(names), Ok(mut sums)) = (times.names.lock(), times.sums.lock()) else {
        return;
    };
    for (i, pair) in stamps.chunks_exact(2).enumerate() {
        let Some(name) = names.get(i) else { break };
        let ms = pair[1].saturating_sub(pair[0]) as f64 * f64::from(times.period_ns) / 1.0e6;
        let entry = sums.1.entry(format!("{i:02} {name}")).or_insert((0.0, 0));
        entry.0 += ms;
        entry.1 += 1;
    }
    sums.0 += 1;
    if sums.0 >= REPORT_EVERY {
        let total: f64 = sums.1.values().map(|(t, n)| t / f64::from(*n)).sum();
        let mut lines = vec![format!("pass times, {total:.2} ms in passes:")];
        for (name, (t, n)) in &sums.1 {
            lines.push(format!("  {:6.3} ms  {name}", t / f64::from(*n)));
        }
        eprintln!("{}", lines.join("\n"));
        *sums = (0, BTreeMap::new());
    }
}
