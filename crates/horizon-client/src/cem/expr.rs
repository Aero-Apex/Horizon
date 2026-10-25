//! The expressions of a custom entity model's `animations` (OptiFine's CEM format), read the
//! way Entity Model Features (LGPL-3.0, see THIRD_PARTY_NOTICES.md) reads them: numbers,
//! booleans as `+inf` / `-inf`, `* / %` before `+ -` before the comparisons before
//! `&&` / `||` (the last two on one level, left to right), unary `-` and `!`, and the
//! functions the packs use.

use std::cell::Cell;

pub const TRUE: f32 = f32::INFINITY;
pub const FALSE: f32 = f32::NEG_INFINITY;

pub fn from_bool(b: bool) -> f32 {
    if b { TRUE } else { FALSE }
}

/// Any positive value is true (EMF accepts only the two infinities).
pub fn to_bool(v: f32) -> bool {
    v > 0.0
}

/// A transform or flag of a model part: `head.rx`, `body.visible`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartProp {
    Tx,
    Ty,
    Tz,
    Rx,
    Ry,
    Rz,
    Sx,
    Sy,
    Sz,
    Visible,
    VisibleBoxes,
}

impl PartProp {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "tx" => Self::Tx,
            "ty" => Self::Ty,
            "tz" => Self::Tz,
            "rx" => Self::Rx,
            "ry" => Self::Ry,
            "rz" => Self::Rz,
            "sx" => Self::Sx,
            "sy" => Self::Sy,
            "sz" => Self::Sz,
            "visible" => Self::Visible,
            "visible_boxes" => Self::VisibleBoxes,
            _ => return None,
        })
    }

    pub fn is_bool(self) -> bool {
        matches!(self, Self::Visible | Self::VisibleBoxes)
    }
}

/// The variables an entity provides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    LimbSwing,
    LimbSpeed,
    Age,
    FrameTime,
    FrameCounter,
    HeadYaw,
    HeadPitch,
    SwingProgress,
    HurtTime,
    DeathTime,
    Health,
    MaxHealth,
    Id,
    Time,
    DayTime,
    DayCount,
    RuleIndex,
    Dimension,
    PosX,
    PosY,
    PosZ,
    RotX,
    RotY,
    PlayerPosX,
    PlayerPosY,
    PlayerPosZ,
    PlayerRotX,
    PlayerRotY,
    MoveForward,
    MoveStrafing,
    HeightAboveGround,
    FluidDepth,
    FluidDepthDown,
    FluidDepthUp,
    AngerTime,
    AngerTimeStart,
    Distance,
    Nan,
    // Booleans.
    IsChild,
    IsInWater,
    IsRiding,
    IsOnGround,
    IsAggressive,
    IsHurt,
    IsSitting,
    IsRidden,
    IsOnShoulder,
    IsInLava,
    IsAlive,
    IsTamed,
    IsSprinting,
    IsSneaking,
    IsBurning,
    IsWet,
    IsGlowing,
    IsInvisible,
    IsClimbing,
    IsJumping,
    IsSwimming,
    IsGliding,
    IsBlocking,
    IsCrawling,
    IsPaused,
    IsHovered,
    IsInGui,
    IsInHand,
    IsInItemFrame,
    IsInGround,
    IsOnHead,
    IsFirstPersonHand,
    IsRightHanded,
    IsSwingingRightArm,
    IsSwingingLeftArm,
    IsHoldingItemRight,
    IsHoldingItemLeft,
    IsUsingItem,
    IsPlayerFirstPerson,
    IsPlayerThirdPerson,
    IsPlayerThirdPersonReversed,
}

impl Builtin {
    pub fn from_name(name: &str) -> Option<Self> {
        use Builtin::*;
        Some(match name {
            "limb_swing" => LimbSwing,
            "limb_speed" => LimbSpeed,
            "age" => Age,
            "frame_time" => FrameTime,
            "frame_counter" => FrameCounter,
            "head_yaw" => HeadYaw,
            "head_pitch" => HeadPitch,
            "swing_progress" => SwingProgress,
            "hurt_time" => HurtTime,
            "death_time" => DeathTime,
            "health" => Health,
            "max_health" => MaxHealth,
            "id" => Id,
            "time" => Time,
            "day_time" => DayTime,
            "day_count" => DayCount,
            "rule_index" => RuleIndex,
            "dimension" => Dimension,
            "pos_x" => PosX,
            "pos_y" => PosY,
            "pos_z" => PosZ,
            "rot_x" => RotX,
            "rot_y" => RotY,
            "player_pos_x" => PlayerPosX,
            "player_pos_y" => PlayerPosY,
            "player_pos_z" => PlayerPosZ,
            "player_rot_x" => PlayerRotX,
            "player_rot_y" => PlayerRotY,
            "move_forward" => MoveForward,
            "move_strafing" => MoveStrafing,
            "height_above_ground" => HeightAboveGround,
            "fluid_depth" => FluidDepth,
            "fluid_depth_down" => FluidDepthDown,
            "fluid_depth_up" => FluidDepthUp,
            "anger_time" => AngerTime,
            "anger_time_start" => AngerTimeStart,
            "distance" => Distance,
            "nan" => Nan,
            "is_child" => IsChild,
            "is_in_water" => IsInWater,
            "is_riding" => IsRiding,
            "is_on_ground" => IsOnGround,
            "is_aggressive" => IsAggressive,
            "is_hurt" => IsHurt,
            "is_sitting" => IsSitting,
            "is_ridden" => IsRidden,
            "is_on_shoulder" => IsOnShoulder,
            "is_in_lava" => IsInLava,
            "is_alive" => IsAlive,
            "is_tamed" => IsTamed,
            "is_sprinting" => IsSprinting,
            "is_sneaking" => IsSneaking,
            "is_burning" => IsBurning,
            "is_wet" => IsWet,
            "is_glowing" => IsGlowing,
            "is_invisible" => IsInvisible,
            "is_climbing" => IsClimbing,
            "is_jumping" => IsJumping,
            "is_swimming" => IsSwimming,
            "is_gliding" => IsGliding,
            "is_blocking" => IsBlocking,
            "is_crawling" => IsCrawling,
            "is_paused" => IsPaused,
            "is_hovered" => IsHovered,
            "is_in_gui" => IsInGui,
            "is_in_hand" => IsInHand,
            "is_in_item_frame" => IsInItemFrame,
            "is_in_ground" => IsInGround,
            "is_on_head" => IsOnHead,
            "is_first_person_hand" => IsFirstPersonHand,
            "is_right_handed" => IsRightHanded,
            "is_swinging_right_arm" => IsSwingingRightArm,
            "is_swinging_left_arm" => IsSwingingLeftArm,
            "is_holding_item_right" => IsHoldingItemRight,
            "is_holding_item_left" => IsHoldingItemLeft,
            "is_using_item" => IsUsingItem,
            "is_player_first_person" => IsPlayerFirstPerson,
            "is_player_third_person" => IsPlayerThirdPerson,
            "is_player_third_person_reversed" => IsPlayerThirdPersonReversed,
            _ => return None,
        })
    }
}

/// What an expression reads while it runs.
pub trait Ctx {
    fn builtin(&mut self, b: Builtin) -> f32;
    fn part(&mut self, part: usize, prop: PartProp) -> f32;
    fn var(&mut self, slot: u32) -> f32;
    fn global(&mut self, slot: u32) -> f32;
}

/// What a parsed expression binds names to.
pub trait Resolver {
    /// The part an expression's `name.tx` means, or none.
    fn part(&mut self, id: &str) -> Option<usize>;
    /// `var.x` / `varb.x`, by their full key.
    fn entity_var(&mut self, key: &str) -> u32;
    /// `global_var.x` / `global_varb.x`, by their full key.
    fn global_var(&mut self, key: &str) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Func {
    Sin,
    Cos,
    Asin,
    Acos,
    Tan,
    Atan,
    Abs,
    Floor,
    Ceil,
    Round,
    Log,
    Exp,
    Torad,
    Todeg,
    Frac,
    Signum,
    Sqrt,
    Fmod,
    Pow,
    Atan2,
    Clamp,
    Lerp,
    Max,
    Min,
    Random,
    If,
    Between,
    Equals,
    In,
    WrapDeg,
    WrapRad,
    DegDiff,
    RadDiff,
}

impl Func {
    fn from_name(name: &str) -> Option<(Self, std::ops::RangeInclusive<usize>)> {
        use Func::*;
        Some(match name {
            "sin" => (Sin, 1..=1),
            "cos" => (Cos, 1..=1),
            "asin" => (Asin, 1..=1),
            "acos" => (Acos, 1..=1),
            "tan" => (Tan, 1..=1),
            "atan" => (Atan, 1..=1),
            "abs" => (Abs, 1..=1),
            "floor" => (Floor, 1..=1),
            "ceil" => (Ceil, 1..=1),
            "round" => (Round, 1..=1),
            "log" => (Log, 1..=1),
            "exp" => (Exp, 1..=1),
            "torad" => (Torad, 1..=1),
            "todeg" => (Todeg, 1..=1),
            "frac" => (Frac, 1..=1),
            "signum" => (Signum, 1..=1),
            "sqrt" => (Sqrt, 1..=1),
            "fmod" => (Fmod, 2..=2),
            "pow" => (Pow, 2..=2),
            "atan2" => (Atan2, 2..=2),
            "clamp" => (Clamp, 3..=3),
            "lerp" => (Lerp, 3..=3),
            "max" => (Max, 2..=usize::MAX),
            "min" => (Min, 2..=usize::MAX),
            "random" => (Random, 0..=1),
            "if" => (If, 3..=usize::MAX),
            "between" => (Between, 3..=3),
            "equals" => (Equals, 3..=3),
            "in" => (In, 2..=usize::MAX),
            "wrapdeg" => (WrapDeg, 1..=1),
            "wraprad" => (WrapRad, 1..=1),
            "degdiff" => (DegDiff, 2..=2),
            "raddiff" => (RadDiff, 2..=2),
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug)]
enum Node {
    Const(f32),
    Builtin(Builtin),
    Part(usize, PartProp),
    Var(u32),
    Global(u32),
    Neg(Box<Node>),
    Not(Box<Node>),
    Bin(Op, Box<Node>, Box<Node>),
    Call(Func, Vec<Node>),
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Num(f32),
    Ident(String),
    LParen,
    RParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    AndAnd,
    OrOr,
    Bang,
}

fn lex(text: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            ',' => {
                out.push(Token::Comma);
                i += 1;
            }
            '+' => {
                out.push(Token::Plus);
                i += 1;
            }
            '-' => {
                out.push(Token::Minus);
                i += 1;
            }
            '*' => {
                out.push(Token::Star);
                i += 1;
            }
            '/' => {
                out.push(Token::Slash);
                i += 1;
            }
            '%' => {
                out.push(Token::Percent);
                i += 1;
            }
            '<' | '>' | '=' | '!' | '&' | '|' => {
                let two = next == Some('=') && matches!(c, '<' | '>' | '=' | '!');
                let double = next == Some(c) && matches!(c, '&' | '|' | '=');
                let token = match (c, two || double) {
                    ('<', true) => Token::Le,
                    ('<', false) => Token::Lt,
                    ('>', true) => Token::Ge,
                    ('>', false) => Token::Gt,
                    ('=', _) => Token::EqEq,
                    ('!', true) => Token::Ne,
                    ('!', false) => Token::Bang,
                    ('&', _) => Token::AndAnd,
                    _ => Token::OrOr,
                };
                // `==`, `!=`, `<=`, `>=`, `&&`, `||` take two characters; a lone
                // `=`, `&` or `|` stands for the pair.
                i += if two || double { 2 } else { 1 };
                out.push(token);
            }
            c if c.is_ascii_digit() || (c == '.' && next.is_some_and(|n| n.is_ascii_digit())) => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                if matches!(chars.get(i), Some('e' | 'E')) {
                    let mut j = i + 1;
                    if matches!(chars.get(j), Some('+' | '-')) {
                        j += 1;
                    }
                    if chars.get(j).is_some_and(|d| d.is_ascii_digit()) {
                        while j < chars.len() && chars[j].is_ascii_digit() {
                            j += 1;
                        }
                        i = j;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                out.push(Token::Num(
                    s.parse().map_err(|_| format!("bad number {s:?}"))?,
                ));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '.' | ':'))
                {
                    i += 1;
                }
                out.push(Token::Ident(chars[start..i].iter().collect()));
            }
            c => return Err(format!("unexpected {c:?}")),
        }
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    resolver: &'a mut dyn Resolver,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn take(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.at).cloned();
        self.at += 1;
        t
    }

    fn logical(&mut self) -> Result<Node, String> {
        let mut left = self.comparison()?;
        loop {
            let op = match self.peek() {
                Some(Token::AndAnd) => Op::And,
                Some(Token::OrOr) => Op::Or,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.comparison()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn comparison(&mut self) -> Result<Node, String> {
        let mut left = self.additive()?;
        loop {
            let op = match self.peek() {
                Some(Token::EqEq) => Op::Eq,
                Some(Token::Ne) => Op::Ne,
                Some(Token::Lt) => Op::Lt,
                Some(Token::Le) => Op::Le,
                Some(Token::Gt) => Op::Gt,
                Some(Token::Ge) => Op::Ge,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.additive()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn additive(&mut self) -> Result<Node, String> {
        let mut left = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => Op::Add,
                Some(Token::Minus) => Op::Sub,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.multiplicative()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn multiplicative(&mut self) -> Result<Node, String> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => Op::Mul,
                Some(Token::Slash) => Op::Div,
                Some(Token::Percent) => Op::Rem,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.unary()?;
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Node, String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.at += 1;
                Ok(match self.unary()? {
                    Node::Const(c) => Node::Const(-c),
                    inner => Node::Neg(Box::new(inner)),
                })
            }
            Some(Token::Plus) => {
                self.at += 1;
                self.unary()
            }
            Some(Token::Bang) => {
                self.at += 1;
                Ok(Node::Not(Box::new(self.unary()?)))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Node, String> {
        match self.take() {
            Some(Token::Num(n)) => Ok(Node::Const(n)),
            Some(Token::LParen) => {
                let inner = self.logical()?;
                match self.take() {
                    Some(Token::RParen) => Ok(inner),
                    other => Err(format!("expected ')', found {other:?}")),
                }
            }
            Some(Token::Ident(name)) => {
                if self.peek() == Some(&Token::LParen) {
                    self.at += 1;
                    let mut args = Vec::new();
                    if self.peek() == Some(&Token::RParen) {
                        self.at += 1;
                    } else {
                        loop {
                            args.push(self.logical()?);
                            match self.take() {
                                Some(Token::Comma) => {}
                                Some(Token::RParen) => break,
                                other => {
                                    return Err(format!("expected ',' or ')', found {other:?}"));
                                }
                            }
                        }
                    }
                    let (func, arity) =
                        Func::from_name(&name).ok_or_else(|| format!("unknown function {name}"))?;
                    if !arity.contains(&args.len()) {
                        return Err(format!(
                            "{name}() takes {arity:?} arguments, got {}",
                            args.len()
                        ));
                    }
                    if func == Func::If && args.len() % 2 == 0 {
                        return Err("if() needs an odd number of arguments".into());
                    }
                    Ok(Node::Call(func, args))
                } else {
                    self.variable(&name)
                }
            }
            other => Err(format!("unexpected {other:?}")),
        }
    }

    fn variable(&mut self, name: &str) -> Result<Node, String> {
        match name {
            "pi" => return Ok(Node::Const(std::f32::consts::PI)),
            "e" => return Ok(Node::Const(std::f32::consts::E)),
            "true" => return Ok(Node::Const(TRUE)),
            "false" => return Ok(Node::Const(FALSE)),
            _ => {}
        }
        if let Some(b) = Builtin::from_name(name) {
            return Ok(Node::Builtin(b));
        }
        let Some((owner, field)) = name.rsplit_once('.') else {
            return Err(format!("unknown variable {name}"));
        };
        match owner {
            "var" | "varb" => Ok(Node::Var(self.resolver.entity_var(name))),
            "global_var" | "global_varb" => Ok(Node::Global(self.resolver.global_var(name))),
            "render" => Ok(Node::Const(0.0)),
            _ => {
                let prop =
                    PartProp::from_name(field).ok_or_else(|| format!("unknown variable {name}"))?;
                Ok(match self.resolver.part(owner) {
                    Some(part) => Node::Part(part, prop),
                    // EMF reads a part it cannot find as 0 (false).
                    None => Node::Const(if prop.is_bool() { FALSE } else { 0.0 }),
                })
            }
        }
    }
}

thread_local! {
    static RANDOM: Cell<u64> = const { Cell::new(0x9E37_79B9_7F4A_7C15) };
}

fn unseeded_random() -> f32 {
    RANDOM.with(|state| {
        let mut x = state.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        state.set(x);
        (x >> 40) as f32 / (1u64 << 24) as f32
    })
}

/// OptiFine's `random(seed)`: a hash of the seed's bits, in [0, 1).
pub fn seeded_random(seed: f32) -> f32 {
    let mut x = seed.to_bits() as i32;
    x = x ^ 61 ^ (x >> 16);
    x = x.wrapping_add(x << 3);
    x ^= x >> 4;
    x = x.wrapping_mul(668_265_261);
    x ^= x >> 15;
    x.wrapping_abs() as f32 / 2_147_483_648.0
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut d = degrees % 360.0;
    if d >= 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

impl Node {
    fn eval(&self, ctx: &mut dyn Ctx) -> f32 {
        match self {
            Node::Const(c) => *c,
            Node::Builtin(b) => ctx.builtin(*b),
            Node::Part(part, prop) => ctx.part(*part, *prop),
            Node::Var(slot) => ctx.var(*slot),
            Node::Global(slot) => ctx.global(*slot),
            Node::Neg(n) => -n.eval(ctx),
            Node::Not(n) => from_bool(!to_bool(n.eval(ctx))),
            Node::Bin(op, a, b) => {
                let a = a.eval(ctx);
                let b = b.eval(ctx);
                match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => a / b,
                    Op::Rem => a % b,
                    Op::And => from_bool(to_bool(a) && to_bool(b)),
                    Op::Or => from_bool(to_bool(a) || to_bool(b)),
                    Op::Eq => from_bool(a == b),
                    Op::Ne => from_bool(a != b),
                    Op::Lt => from_bool(a < b),
                    Op::Le => from_bool(a <= b),
                    Op::Gt => from_bool(a > b),
                    Op::Ge => from_bool(a >= b),
                }
            }
            Node::Call(func, args) => call(*func, args, ctx),
        }
    }
}

fn call(func: Func, args: &[Node], ctx: &mut dyn Ctx) -> f32 {
    // `if` evaluates only the branch it takes.
    if func == Func::If {
        let pairs = args.len() / 2;
        for i in 0..pairs {
            if to_bool(args[2 * i].eval(ctx)) {
                return args[2 * i + 1].eval(ctx);
            }
        }
        return args[args.len() - 1].eval(ctx);
    }
    let mut a = [0.0f32; 3];
    let fixed = args.len().min(3);
    for (slot, arg) in a.iter_mut().zip(args.iter()).take(fixed) {
        *slot = arg.eval(ctx);
    }
    let x = a[0];
    match func {
        Func::Sin => x.sin(),
        Func::Cos => x.cos(),
        Func::Asin => x.asin(),
        Func::Acos => x.acos(),
        Func::Tan => x.tan(),
        Func::Atan => x.atan(),
        Func::Abs => x.abs(),
        Func::Floor => x.floor(),
        Func::Ceil => x.ceil(),
        Func::Round => (x + 0.5).floor(),
        Func::Log => x.ln(),
        Func::Exp => x.exp(),
        Func::Torad => x.to_radians(),
        Func::Todeg => x.to_degrees(),
        Func::Frac => x - x.floor(),
        Func::Signum => {
            if x == 0.0 {
                0.0
            } else {
                x.signum()
            }
        }
        Func::Sqrt => x.sqrt(),
        // EMF: `Math.floorMod((int) a, (int) b)`.
        Func::Fmod => {
            let (n, d) = (x as i32, a[1] as i32);
            if d == 0 {
                f32::NAN
            } else {
                let m = n.wrapping_rem(d);
                (if m != 0 && (m < 0) != (d < 0) {
                    m + d
                } else {
                    m
                }) as f32
            }
        }
        Func::Pow => x.powf(a[1]),
        Func::Atan2 => x.atan2(a[1]),
        Func::Clamp => {
            if x < a[1] {
                a[1]
            } else {
                x.min(a[2])
            }
        }
        // `Mth.lerp(delta, start, end)`.
        Func::Lerp => a[1] + x * (a[2] - a[1]),
        Func::Max => args[1..].iter().fold(x, |m, n| {
            let v = n.eval(ctx);
            if v > m { v } else { m }
        }),
        Func::Min => args[1..].iter().fold(x, |m, n| {
            let v = n.eval(ctx);
            if v < m { v } else { m }
        }),
        Func::Random => {
            if args.is_empty() {
                unseeded_random()
            } else {
                seeded_random(x)
            }
        }
        Func::Between => from_bool(x >= a[1] && x <= a[2]),
        Func::Equals => from_bool((a[1] - x).abs() <= a[2]),
        Func::In => {
            let rest: Vec<f32> = args[1..].iter().map(|n| n.eval(ctx)).collect();
            from_bool(rest.contains(&x))
        }
        Func::WrapDeg => wrap_degrees(x),
        Func::WrapRad => wrap_degrees(x.to_degrees()).to_radians(),
        Func::DegDiff => {
            let d = (a[1] - x).abs() % 360.0;
            if d > 180.0 { 360.0 - d } else { d }
        }
        Func::RadDiff => {
            let d = (a[1] - x).to_degrees().abs() % 360.0;
            (if d > 180.0 { 360.0 - d } else { d }).to_radians()
        }
        Func::If => unreachable!("handled above"),
    }
}

/// A parsed expression.
#[derive(Clone, Debug)]
pub struct Expr(Node);

impl Expr {
    pub fn parse(text: &str, resolver: &mut dyn Resolver) -> Result<Self, String> {
        let tokens = lex(text)?;
        if tokens.is_empty() {
            return Err("empty expression".into());
        }
        let mut parser = Parser {
            tokens,
            at: 0,
            resolver,
        };
        let node = parser.logical()?;
        if parser.at < parser.tokens.len() {
            return Err(format!("unexpected {:?}", parser.tokens[parser.at]));
        }
        Ok(Self(node))
    }

    pub fn eval(&self, ctx: &mut dyn Ctx) -> f32 {
        self.0.eval(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Names {
        parts: Vec<String>,
        vars: Vec<String>,
    }
    impl Resolver for Names {
        fn part(&mut self, id: &str) -> Option<usize> {
            Some(match self.parts.iter().position(|p| p == id) {
                Some(i) => i,
                None => {
                    self.parts.push(id.to_owned());
                    self.parts.len() - 1
                }
            })
        }
        fn entity_var(&mut self, key: &str) -> u32 {
            match self.vars.iter().position(|v| v == key) {
                Some(i) => i as u32,
                None => {
                    self.vars.push(key.to_owned());
                    self.vars.len() as u32 - 1
                }
            }
        }
        fn global_var(&mut self, key: &str) -> u32 {
            self.entity_var(key)
        }
    }

    #[derive(Default)]
    struct Values {
        builtins: HashMap<Builtin, f32>,
        parts: HashMap<(usize, PartProp), f32>,
        vars: Vec<f32>,
    }
    impl Ctx for Values {
        fn builtin(&mut self, b: Builtin) -> f32 {
            self.builtins.get(&b).copied().unwrap_or(0.0)
        }
        fn part(&mut self, part: usize, prop: PartProp) -> f32 {
            self.parts.get(&(part, prop)).copied().unwrap_or(0.0)
        }
        fn var(&mut self, slot: u32) -> f32 {
            self.vars.get(slot as usize).copied().unwrap_or(0.0)
        }
        fn global(&mut self, slot: u32) -> f32 {
            self.var(slot)
        }
    }

    fn value(text: &str) -> f32 {
        let mut names = Names::default();
        Expr::parse(text, &mut names)
            .unwrap_or_else(|e| panic!("{text}: {e}"))
            .eval(&mut Values::default())
    }

    #[test]
    fn arithmetic_follows_the_usual_order() {
        assert_eq!(value("1+2*3"), 7.0);
        assert_eq!(value("(1+2)*3"), 9.0);
        assert_eq!(value("10-4-3"), 3.0);
        assert_eq!(value("-2*3"), -6.0);
        assert_eq!(value("2*-3"), -6.0);
        assert_eq!(value("7%4"), 3.0);
        assert!((value("torad(180)") - std::f32::consts::PI).abs() < 1e-6);
        assert!((value("-sin(pi/2)") + 1.0).abs() < 1e-6);
    }

    #[test]
    fn booleans_are_infinities_and_logic_is_flat() {
        assert_eq!(value("1<2"), TRUE);
        assert_eq!(value("1>2"), FALSE);
        assert_eq!(value("!(1>2)"), TRUE);
        assert_eq!(value("1<2 && 3<2"), FALSE);
        assert_eq!(value("1<2 || 3<2"), TRUE);
        assert_eq!(value("true && !false"), TRUE);
        assert_eq!(value("2==2"), TRUE);
        assert_eq!(value("2!=2"), FALSE);
        assert_eq!(value("1<=1 && 2>=3"), FALSE);
        // `&&` and `||` share a level and group from the left: (true || false) && false.
        assert_eq!(value("true || false && false"), FALSE);
    }

    #[test]
    fn if_takes_chains_and_clamp_and_between() {
        assert_eq!(value("if(1<2, 5, 6)"), 5.0);
        assert_eq!(value("if(1>2, 5, 6)"), 6.0);
        assert_eq!(value("if(1>2, 1, 2>3, 2, 3)"), 3.0);
        assert_eq!(value("if(1>2, 1, 3>2, 2, 3)"), 2.0);
        assert_eq!(value("clamp(5, 0, 3)"), 3.0);
        assert_eq!(value("clamp(-5, 0, 3)"), 0.0);
        assert_eq!(value("max(1, 5, 3)"), 5.0);
        assert_eq!(value("min(4, 2, 3)"), 2.0);
        assert_eq!(value("between(2, 1, 3)"), TRUE);
        assert_eq!(value("between(4, 1, 3)"), FALSE);
        assert_eq!(value("equals(1.0, 1.05, 0.1)"), TRUE);
        assert_eq!(value("in(3, 1, 2, 3)"), TRUE);
        assert_eq!(value("in(4, 1, 2, 3)"), FALSE);
    }

    #[test]
    fn seeded_random_matches_optifine() {
        // Stable across runs and spread over [0, 1).
        let a = seeded_random(1.0);
        let b = seeded_random(2.0);
        assert!((0.0..1.0).contains(&a) && (0.0..1.0).contains(&b));
        assert_ne!(a, b);
        assert_eq!(seeded_random(1.0), a);
        assert_eq!(value("random(1)"), a);
    }

    #[test]
    fn parts_and_variables_resolve() {
        let mut names = Names::default();
        let e = Expr::parse(
            "body.ty + var.x * 2 + (right_arm.rx - left_leg.rx)",
            &mut names,
        )
        .unwrap();
        assert_eq!(names.parts, ["body", "right_arm", "left_leg"]);
        assert_eq!(names.vars, ["var.x"]);
        let mut v = Values::default();
        v.parts.insert((0, PartProp::Ty), 3.5);
        v.parts.insert((1, PartProp::Rx), 1.0);
        v.vars = vec![4.0];
        assert_eq!(e.eval(&mut v), 3.5 + 8.0 + 1.0);
    }

    #[test]
    fn rejects_what_it_cannot_read() {
        let mut names = Names::default();
        assert!(Expr::parse("sin(", &mut names).is_err());
        assert!(Expr::parse("wobble(1)", &mut names).is_err());
        assert!(Expr::parse("1 +", &mut names).is_err());
        assert!(Expr::parse("nonsense", &mut names).is_err());
        assert!(Expr::parse("if(1,2)", &mut names).is_err());
    }

    #[test]
    fn a_fresh_animations_line_evaluates() {
        // From the iron golem animations of Fresh Animations.
        let line = "3.5 +( -sin(pi/3 +var.ls*2 -sin(pi/6 +var.ls*2)/1.7 )*1.5 +1 )*clamp(limb_speed*4, 0, 1)";
        let mut names = Names::default();
        let e = Expr::parse(line, &mut names).unwrap();
        assert_eq!(e.eval(&mut Values::default()), 3.5);
    }
}
