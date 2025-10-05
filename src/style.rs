use ratatui::style::Color;

#[derive(Default)]
pub struct UIStyle {
    pub list_style: ListStyle,
    pub action_style: ActionStyle,
    pub seeker_style: SeekerStyle,
    pub volume_style: VolumeStyle,
    pub lyrics_style: LyricsStyle,
    pub playlists_style: PlaylistsStyle,
}

pub struct ListStyle {
    pub hilight_color: Color,
    pub active_region_color: Color,
    pub active_search_region_color: Color,
    pub active_after_search_region_color: Color,
    pub active_command_region_color: Color,
    pub passive_region_color: Color,
    pub selector: String,
    pub playing_selector: String,
    pub playing_region_color: Color,
}

pub struct SeekerStyle {
    pub active_region_color: Color,
    pub passive_region_color: Color,
    pub fg_seeker_color: Color,
    pub bg_seeker_color: Color,
}

pub struct VolumeStyle {
    pub active_region_color: Color,
    pub passive_region_color: Color,
    pub fg_volume_color: Color,
    pub bg_volume_color: Color,
}

pub struct PlaylistsStyle {
    pub active_region_color: Color,
    pub passive_region_color: Color,
    pub selector: String,
}

pub struct LyricsStyle {
    pub active_region_color: Color,
    pub passive_region_color: Color,
    pub selector: String,
}

pub struct ActionStyle {
    pub hilight_color: Color,
    pub active_region_color: Color,
    pub passive_region_color: Color,
}

impl Default for ListStyle {
    fn default() -> Self {
        ListStyle {
            hilight_color: Color::default(),
            playing_region_color: Color::Gray,
            active_region_color: Color::Magenta,
            active_command_region_color: Color::Cyan,
            active_after_search_region_color: Color::Cyan,
            active_search_region_color: Color::DarkGray,
            passive_region_color: Color::default(),
            selector: String::from(">>"),
            playing_selector: String::from("*"),
        }
    }
}

impl UIStyle {
    #[allow(dead_code)]
    pub fn new(
        list_style: ListStyle,
        action_style: ActionStyle,
        seeker_style: SeekerStyle,
        volume_style: VolumeStyle,
        lyrics_style: LyricsStyle,
        playlists_style: PlaylistsStyle,
    ) -> Self {
        UIStyle {
            list_style,
            action_style,
            seeker_style,
            volume_style,
            lyrics_style,
            playlists_style,
        }
    }
}

impl Default for SeekerStyle {
    fn default() -> Self {
        SeekerStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            fg_seeker_color: Color::Gray,
            bg_seeker_color: Color::Black,
        }
    }
}

impl Default for VolumeStyle {
    fn default() -> Self {
        VolumeStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            fg_volume_color: Color::Gray,
            bg_volume_color: Color::Black,
        }
    }
}

impl Default for ActionStyle {
    fn default() -> Self {
        ActionStyle {
            hilight_color: Color::Yellow,
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
        }
    }
}

impl Default for LyricsStyle {
    fn default() -> Self {
        LyricsStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            selector: String::from(">>"),
        }
    }
}

impl Default for PlaylistsStyle {
    fn default() -> Self {
        Self {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            selector: String::from(">>"),
        }
    }
}
