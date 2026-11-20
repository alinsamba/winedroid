// cite: frameworks-base ResourceTypes.h#Res_GETPACKAGE
pub(crate) fn is_framework_package_resource_id(resource_id: u32) -> bool {
    resource_id >> 24 == 0x01
}

// cite: frameworks-base public-final.xml#Theme.Material, Theme.Holo, Theme.DeviceDefault
// cite: frameworks-base themes_material.xml#windowActionBar, windowNoTitle
// cite: frameworks-base config_material.xml#config_windowActionBarSupported
pub(crate) fn framework_theme_has_decor_action_bar(resource_id: u32) -> Option<bool> {
    if resource_id >> 16 != 0x0103 {
        return None;
    }

    Some(matches!(
        resource_id,
        0x0103_006b
            | 0x0103_006e
            | 0x0103_0119
            | 0x0103_0128
            | 0x0103_012b
            | 0x0103_013f
            | 0x0103_0223..=0x0103_0224
            | 0x0103_0233
            | 0x0103_0237..=0x0103_0238
            | 0x0103_02d5
            | 0x0103_02e2
    ))
}

// cite: frameworks-base public-final.xml#Theme.Dialog, Theme.Material.Light.Dialog, Theme.Material.Light.DialogWhenLarge.NoActionBar
pub(crate) fn is_floating_window_theme(resource_id: u32) -> bool {
    matches!(
        resource_id,
        0x0103_000b
            | 0x0103_0059
            | 0x0103_005a
            | 0x0103_006f..=0x0103_007c
            | 0x0103_012e..=0x0103_013b
            | 0x0103_0225..=0x0103_022c
            | 0x0103_0232
            | 0x0103_0239..=0x0103_0240
            | 0x0103_0245
            | 0x0103_02d1
            | 0x0103_02d2
            | 0x0103_02d8
    )
}

// cite: frameworks-base public-final.xml#Theme.Material.Light, Theme.Material.Light.NoActionBar
// cite: frameworks-base themes_material.xml#windowBackground
// cite: frameworks-base colors_material.xml#background_material_light, background_material_dark
pub(crate) fn framework_theme_window_background(resource_id: u32) -> Option<u32> {
    match resource_id {
        0x0103_0237 | 0x0103_0238 | 0x0103_0241 | 0x0103_0242 | 0x0103_0243 | 0x0103_02d5 => {
            Some(0xfffa_fafa)
        }
        0x0103_0224 | 0x0103_022e | 0x0103_022f | 0x0103_0230 => Some(0xff30_3030),
        _ => None,
    }
}

// cite: frameworks-base themes_material.xml#textColorPrimary
// cite: frameworks-base colors_material.xml#primary_text_default_material_light, primary_text_default_material_dark
pub(crate) fn framework_theme_text_color_primary(resource_id: u32) -> Option<u32> {
    match resource_id {
        0x0103_0237 | 0x0103_0238 | 0x0103_0241 | 0x0103_0242 | 0x0103_0243 | 0x0103_02d5 => {
            Some(0xde00_0000)
        }
        0x0103_0224 | 0x0103_022e | 0x0103_022f | 0x0103_0230 => Some(0xffff_ffff),
        _ => None,
    }
}

// cite: frameworks-base attrs.xml#actionBarSize
// cite: frameworks-base dimens_material.xml#action_bar_default_height_material
pub(crate) fn framework_attr_dimension_px(attribute_id: u32) -> Option<i32> {
    match attribute_id {
        0x0101_02eb => Some(56),
        _ => None,
    }
}

pub(crate) fn color_resource(resource_id: u32) -> Option<u32> {
    match resource_id {
        0x0106_0000 => Some(0xffaa_aaaa),
        0x0106_000b => Some(0xffff_ffff),
        0x0106_000c => Some(0xff00_0000),
        0x0106_000d => Some(0x0000_0000),
        0x0106_000e => Some(0xff00_0000),
        0x0106_000f => Some(0xffff_ffff),
        0x0106_001c => Some(0x0000_0000),
        _ => framework_system_color(resource_id),
    }
}

fn framework_system_color(resource_id: u32) -> Option<u32> {
    color_from_table(resource_id, 0x0106_001d, &SYSTEM_NEUTRAL1)
        .or_else(|| color_from_table(resource_id, 0x0106_002a, &SYSTEM_NEUTRAL2))
        .or_else(|| color_from_table(resource_id, 0x0106_0037, &SYSTEM_ACCENT1))
        .or_else(|| color_from_table(resource_id, 0x0106_0044, &SYSTEM_ACCENT2))
        .or_else(|| color_from_table(resource_id, 0x0106_0051, &SYSTEM_ACCENT3))
        .or_else(|| color_from_table(resource_id, 0x0106_005e, &SYSTEM_LIGHT_ROLES))
        .or_else(|| color_from_table(resource_id, 0x0106_0089, &SYSTEM_DARK_ROLES))
        .or_else(|| color_from_table(resource_id, 0x0106_00b4, &SYSTEM_FIXED_ROLES))
        .or_else(|| color_from_table(resource_id, 0x0106_00c0, &SYSTEM_OUTLINE_VARIANTS))
        .or_else(|| color_from_table(resource_id, 0x0106_00c5, &SYSTEM_ERROR_TONES))
}

fn color_from_table(resource_id: u32, first_id: u32, colors: &[u32]) -> Option<u32> {
    let index = resource_id.checked_sub(first_id)?;
    colors.get(index as usize).copied()
}

const SYSTEM_ACCENT1: [u32; 13] = [
    0xffff_ffff,
    0xfffe_fbff,
    0xffee_f0ff,
    0xffd9_e2ff,
    0xffb0_c6ff,
    0xff94_aae4,
    0xff7a_90c8,
    0xff60_76ac,
    0xff47_5d92,
    0xff2f_4578,
    0xff15_2e60,
    0xff00_1945,
    0xff00_0000,
];

const SYSTEM_ACCENT2: [u32; 13] = [
    0xffff_ffff,
    0xfffe_fbff,
    0xffee_f0ff,
    0xffdc_e2f9,
    0xffc0_c6dc,
    0xffa4_abc1,
    0xff8a_90a5,
    0xff70_778b,
    0xff57_5e71,
    0xff40_4659,
    0xff2a_3042,
    0xff15_1b2c,
    0xff00_0000,
];

const SYSTEM_ACCENT3: [u32; 13] = [
    0xffff_ffff,
    0xffff_fbff,
    0xffff_ebfa,
    0xfffd_d7fa,
    0xffe0_bbdd,
    0xffc3_a0c1,
    0xffa8_86a6,
    0xff8c_6d8c,
    0xff72_5572,
    0xff59_3d59,
    0xff41_2742,
    0xff2a_122c,
    0xff00_0000,
];

const SYSTEM_NEUTRAL1: [u32; 13] = [
    0xffff_ffff,
    0xfffe_fbff,
    0xfff1_f0f7,
    0xffe2_e2e9,
    0xffc6_c6cd,
    0xffab_abb1,
    0xff90_9097,
    0xff76_777d,
    0xff5d_5e64,
    0xff45_464c,
    0xff2f_3036,
    0xff1a_1b20,
    0xff00_0000,
];

const SYSTEM_NEUTRAL2: [u32; 13] = [
    0xffff_ffff,
    0xfffe_fbff,
    0xfff0_f0fa,
    0xffe1_e2ec,
    0xffc5_c6d0,
    0xffa9_abb4,
    0xff8f_9099,
    0xff75_7780,
    0xff5c_5e67,
    0xff44_464f,
    0xff2e_3038,
    0xff19_1b23,
    0xff00_0000,
];

const SYSTEM_ERROR_TONES: [u32; 13] = [
    0xffff_ffff,
    0xffff_fbf9,
    0xfffc_eeee,
    0xfff9_dedc,
    0xfff2_b8b5,
    0xffec_928e,
    0xffe4_6962,
    0xffdc_362e,
    0xffb3_261e,
    0xff8c_1d18,
    0xff60_1410,
    0xff41_0e0b,
    0xff00_0000,
];

const SYSTEM_LIGHT_ROLES: [u32; 43] = [
    0xff5e_73a9,
    0xffff_ffff,
    0xff2a_4174,
    0xffff_ffff,
    0xff6e_7488,
    0xffff_ffff,
    0xff3c_4255,
    0xffff_ffff,
    0xff8a_6a89,
    0xffff_ffff,
    0xff55_3a55,
    0xffff_ffff,
    0xfffa_f8ff,
    0xff1a_1b20,
    0xfffa_f8ff,
    0xff1a_1b20,
    0xfff4_f3fa,
    0xffff_ffff,
    0xffee_edf4,
    0xffe8_e7ef,
    0xffe2_e2e9,
    0xfffa_f8ff,
    0xffda_d9e0,
    0xffe1_e2ec,
    0xff40_434b,
    0xff5d_5f67,
    0xff8c_0009,
    0xffff_ffff,
    0xffda_342e,
    0xffff_ffff,
    0xffd9_e2ff,
    0xff44_464f,
    0xff00_0000,
    0xffe2_e2e9,
    0xffc5_c6d0,
    0xffe2_e2e9,
    0xffe2_e2e9,
    0xffe2_e2e9,
    0xff60_76ac,
    0xff70_778b,
    0xff8c_6d8c,
    0xff76_777d,
    0xff75_7780,
];

const SYSTEM_DARK_ROLES: [u32; 43] = [
    0xff7a_90c8,
    0xff00_0000,
    0xffb7_caff,
    0xff00_143b,
    0xff8a_90a5,
    0xff00_0000,
    0xffc4_cae1,
    0xff0f_1626,
    0xffa8_86a6,
    0xff00_0000,
    0xffe4_bfe2,
    0xff24_0d26,
    0xff12_1318,
    0xffe2_e2e9,
    0xff12_1318,
    0xfffc_faff,
    0xff1a_1b20,
    0xff0c_0e13,
    0xff1e_1f25,
    0xff28_2a2f,
    0xff33_343a,
    0xff38_393f,
    0xff12_1318,
    0xff44_464f,
    0xffc9_cad4,
    0xffa1_a2ac,
    0xffff_bab1,
    0xff37_0001,
    0xffff_5449,
    0xff00_0000,
    0xff2f_4578,
    0xffc5_c6d0,
    0xffff_ffff,
    0xff1a_1b20,
    0xff44_464f,
    0xff1a_1b20,
    0xff1a_1b20,
    0xff1a_1b20,
    0xff60_76ac,
    0xff70_778b,
    0xff8c_6d8c,
    0xff76_777d,
    0xff75_7780,
];

const SYSTEM_FIXED_ROLES: [u32; 12] = [
    0xff5e_73a9,
    0xff45_5b8f,
    0xffff_ffff,
    0xffff_ffff,
    0xff6e_7488,
    0xff55_5c6f,
    0xffff_ffff,
    0xffff_ffff,
    0xff8a_6a89,
    0xff70_5270,
    0xffff_ffff,
    0xffff_ffff,
];

const SYSTEM_OUTLINE_VARIANTS: [u32; 2] = [0xff79_7a83, 0xff81_838c];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_android_35_system_tonal_colors() {
        assert_eq!(color_resource(0x0106_0047), Some(0xffdc_e2f9));
        assert_eq!(color_resource(0x0106_0020), Some(0xffe2_e2e9));
        assert_eq!(color_resource(0x0106_00c8), Some(0xfff9_dedc));
    }

    #[test]
    fn resolves_android_35_material_role_colors() {
        assert_eq!(color_resource(0x0106_0060), Some(0xff2a_4174));
        assert_eq!(color_resource(0x0106_008b), Some(0xffb7_caff));
        assert_eq!(color_resource(0x0106_00c0), Some(0xff79_7a83));
        assert_eq!(color_resource(0x0106_00b4), Some(0xff5e_73a9));
    }

    #[test]
    fn unsupported_framework_colors_do_not_get_synthesized() {
        assert_eq!(color_resource(0x0106_00c2), None);
        assert_eq!(color_resource(0x0106_ffff), None);
    }
}
