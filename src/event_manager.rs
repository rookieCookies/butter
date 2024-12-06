use tracing::{info, trace};

use crate::math::vector::Vec2;

#[derive(Debug)]
pub struct EventManager {
    sokol_event_queue: Vec<Event>,
}


impl EventManager {
    pub fn new() -> Self {
        let em = Self {
            sokol_event_queue: Vec::new()
        };


        info!("creating event manager ");
        em
    }


    pub fn push_event(&mut self, e: Event) {
        trace!("pushing event {:?}", e);
        self.sokol_event_queue.push(e);
    }


    pub fn event_queue(&self) -> std::slice::Iter<Event> {
        trace!("event queue requested (size: {})", self.sokol_event_queue.len());
        self.sokol_event_queue.iter()
    }


    pub fn clear_queue(&mut self) {
        trace!("clearing event queue");
        self.sokol_event_queue.clear();
    }
}


#[derive(Debug)]
pub enum Event {
    KeyDown(Keycode, bool),
    KeyUp(Keycode, bool),
    Character(char),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseMove {
        abs: Vec2,
        delta: Vec2,
    },
    MouseScroll(Vec2),
    MouseLeave,
    MouseEnter,
    Resized,
    Minimised,
    Restored,
    Focused,
    Unfocused,
    Suspended,
    Resumed,
    QuitRequested,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Keycode {
    Invalid = 0,
    Space = 32,
    Apostrophe = 39,
    Comma = 44,
    Minus = 45,
    Period = 46,
    Slash = 47,
    Num0 = 48,
    Num1 = 49,
    Num2 = 50,
    Num3 = 51,
    Num4 = 52,
    Num5 = 53,
    Num6 = 54,
    Num7 = 55,
    Num8 = 56,
    Num9 = 57,
    Semicolon = 59,
    Equal = 61,
    A = 65,
    B = 66,
    C = 67,
    D = 68,
    E = 69,
    F = 70,
    G = 71,
    H = 72,
    I = 73,
    J = 74,
    K = 75,
    L = 76,
    M = 77,
    N = 78,
    O = 79,
    P = 80,
    Q = 81,
    R = 82,
    S = 83,
    T = 84,
    U = 85,
    V = 86,
    W = 87,
    X = 88,
    Y = 89,
    Z = 90,
    LeftBracket = 91,
    Backslash = 92,
    RightBracket = 93,
    GraveAccent = 96,
    World1 = 161,
    World2 = 162,
    Escape = 256,
    Enter = 257,
    Tab = 258,
    Backspace = 259,
    Insert = 260,
    Delete = 261,
    Right = 262,
    Left = 263,
    Down = 264,
    Up = 265,
    PageUp = 266,
    PageDown = 267,
    Home = 268,
    End = 269,
    CapsLock = 280,
    ScrollLock = 281,
    NumLock = 282,
    PrintScreen = 283,
    Pause = 284,
    F1 = 290,
    F2 = 291,
    F3 = 292,
    F4 = 293,
    F5 = 294,
    F6 = 295,
    F7 = 296,
    F8 = 297,
    F9 = 298,
    F10 = 299,
    F11 = 300,
    F12 = 301,
    F13 = 302,
    F14 = 303,
    F15 = 304,
    F16 = 305,
    F17 = 306,
    F18 = 307,
    F19 = 308,
    F20 = 309,
    F21 = 310,
    F22 = 311,
    F23 = 312,
    F24 = 313,
    F25 = 314,
    Kp0 = 320,
    Kp1 = 321,
    Kp2 = 322,
    Kp3 = 323,
    Kp4 = 324,
    Kp5 = 325,
    Kp6 = 326,
    Kp7 = 327,
    Kp8 = 328,
    Kp9 = 329,
    KpDecimal = 330,
    KpDivide = 331,
    KpMultiply = 332,
    KpSubtract = 333,
    KpAdd = 334,
    KpEnter = 335,
    KpEqual = 336,
    LeftShift = 340,
    LeftControl = 341,
    LeftAlt = 342,
    LeftSuper = 343,
    RightShift = 344,
    RightControl = 345,
    RightAlt = 346,
    RightSuper = 347,
    Menu = 348,
}


impl Keycode {
    pub fn from_sokol(keycode: sokol::app::Keycode) -> Self {
        unsafe { core::mem::transmute(keycode) }
    }


    pub fn from_str(str: &str) -> Option<Self> {
        Some(match str.to_lowercase().as_str() {
            "space" => Self::Space,
            "apostrophe" => Self::Apostrophe,
            "comma" => Self::Comma,
            "minus" => Self::Minus,
            "period" => Self::Period,
            "slash" => Self::Slash,
            "num0" => Self::Num0,
            "num1" => Self::Num1,
            "num2" => Self::Num2,
            "num3" => Self::Num3,
            "num4" => Self::Num4,
            "num5" => Self::Num5,
            "num6" => Self::Num6,
            "num7" => Self::Num7,
            "num8" => Self::Num8,
            "num9" => Self::Num9,
            "semicolon" => Self::Semicolon,
            "equal" => Self::Equal,
            "a" => Self::A,
            "b" => Self::B,
            "c" => Self::C,
            "d" => Self::D,
            "e" => Self::E,
            "f" => Self::F,
            "g" => Self::G,
            "h" => Self::H,
            "i" => Self::I,
            "j" => Self::J,
            "k" => Self::K,
            "l" => Self::L,
            "m" => Self::M,
            "n" => Self::N,
            "o" => Self::O,
            "p" => Self::P,
            "q" => Self::Q,
            "r" => Self::R,
            "s" => Self::S,
            "t" => Self::T,
            "u" => Self::U,
            "v" => Self::V,
            "w" => Self::W,
            "x" => Self::X,
            "y" => Self::Y,
            "z" => Self::Z,
            "leftbracket" => Self::LeftBracket,
            "backslash" => Self::Backslash,
            "rightbracket" => Self::RightBracket,
            "graveaccent" => Self::GraveAccent,
            "world1" => Self::World1,
            "world2" => Self::World2,
            "escape" => Self::Escape,
            "enter" => Self::Enter,
            "tab" => Self::Tab,
            "backspace" => Self::Backspace,
            "insert" => Self::Insert,
            "delete" => Self::Delete,
            "right" => Self::Right,
            "left" => Self::Left,
            "down" => Self::Down,
            "up" => Self::Up,
            "pageup" => Self::PageUp,
            "pagedown" => Self::PageDown,
            "home" => Self::Home,
            "end" => Self::End,
            "capslock" => Self::CapsLock,
            "scrolllock" => Self::ScrollLock,
            "numlock" => Self::NumLock,
            "printscreen" => Self::PrintScreen,
            "pause" => Self::Pause,
            "f1" => Self::F1,
            "f2" => Self::F2,
            "f3" => Self::F3,
            "f4" => Self::F4,
            "f5" => Self::F5,
            "f6" => Self::F6,
            "f7" => Self::F7,
            "f8" => Self::F8,
            "f9" => Self::F9,
            "f10" => Self::F10,
            "f11" => Self::F11,
            "f12" => Self::F12,
            "f13" => Self::F13,
            "f14" => Self::F14,
            "f15" => Self::F15,
            "f16" => Self::F16,
            "f17" => Self::F17,
            "f18" => Self::F18,
            "f19" => Self::F19,
            "f20" => Self::F20,
            "f21" => Self::F21,
            "f22" => Self::F22,
            "f23" => Self::F23,
            "f24" => Self::F24,
            "f25" => Self::F25,
            "kp0" => Self::Kp0,
            "kp1" => Self::Kp1,
            "kp2" => Self::Kp2,
            "kp3" => Self::Kp3,
            "kp4" => Self::Kp4,
            "kp5" => Self::Kp5,
            "kp6" => Self::Kp6,
            "kp7" => Self::Kp7,
            "kp8" => Self::Kp8,
            "kp9" => Self::Kp9,
            "kpdecimal" => Self::KpDecimal,
            "kpdivide" => Self::KpDivide,
            "kpmultiply" => Self::KpMultiply,
            "kpsubtract" => Self::KpSubtract,
            "kpadd" => Self::KpAdd,
            "kpenter" => Self::KpEnter,
            "kpequal" => Self::KpEqual,
            "leftshift" => Self::LeftShift,
            "leftcontrol" => Self::LeftControl,
            "leftalt" => Self::LeftAlt,
            "leftsuper" => Self::LeftSuper,
            "rightshift" => Self::RightShift,
            "rightcontrol" => Self::RightControl,
            "rightalt" => Self::RightAlt,
            "rightsuper" => Self::RightSuper,
            "menu" => Self::Menu,

            _ => return None
        })
    }
}


#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}


impl MouseButton {
    pub fn from_sokol(keycode: sokol::app::Mousebutton) -> Self {
        match keycode {
            sokol::app::Mousebutton::Left => Self::Left,
            sokol::app::Mousebutton::Right => Self::Right,
            sokol::app::Mousebutton::Middle => Self::Middle,
            sokol::app::Mousebutton::Invalid => {
                tracing::error!("invalid mouse button pressed \
                       defaulting to left click");
                Self::Left
            },
        }
    }
}
