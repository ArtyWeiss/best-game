#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum KeyCode {
    Key0,
    Key1,
    Undefined,
}

pub fn to_keycode(code: u32) -> KeyCode {
    match code {
        KEY_0 => KeyCode::Key0,
        KEY_1 => KeyCode::Key1,
        _ => KeyCode::Undefined,
    }
}

pub const KEY_0: u32 = 0x30;
pub const KEY_1: u32 = 0x31;
pub const KEY_2: u32 = 0x32;
pub const KEY_3: u32 = 0x33;
pub const KEY_4: u32 = 0x34;
pub const KEY_5: u32 = 0x35;
pub const KEY_6: u32 = 0x36;
pub const KEY_7: u32 = 0x37;
pub const KEY_8: u32 = 0x38;
pub const KEY_9: u32 = 0x39;
pub const A_KEY: u32 = 0x41;
pub const B_KEY: u32 = 0x42;
pub const C_KEY: u32 = 0x43;
pub const D_KEY: u32 = 0x44;
pub const E_KEY: u32 = 0x45;
pub const F_KEY: u32 = 0x46;
pub const G_KEY: u32 = 0x47;
pub const H_KEY: u32 = 0x48;
pub const I_KEY: u32 = 0x49;
pub const J_KEY: u32 = 0x4A;
pub const K_KEY: u32 = 0x4B;
pub const L_KEY: u32 = 0x4C;
pub const M_KEY: u32 = 0x4D;
pub const N_KEY: u32 = 0x4E;
pub const O_KEY: u32 = 0x4F;
pub const P_KEY: u32 = 0x50;
pub const Q_KEY: u32 = 0x51;
pub const R_KEY: u32 = 0x52;
pub const S_KEY: u32 = 0x53;
pub const T_KEY: u32 = 0x54;
pub const U_KEY: u32 = 0x55;
pub const V_KEY: u32 = 0x56;
pub const W_KEY: u32 = 0x57;
pub const X_KEY: u32 = 0x58;
pub const Y_KEY: u32 = 0x59;
pub const Z_KEY: u32 = 0x5A;
