use async_std::{prelude::*, sync::Arc};
use rustc_hash::FxHashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FixtureParameter {
    Dimmer,
    Red,
    Green,
    Blue,
    CoolWhite,
    Pan,
    Tilt,
    Unused,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct FixtureProfileChannel {
    parameter: FixtureParameter,
    channel: usize,
    #[serde(default = "FixtureProfileChannel::default_min_value")]
    min_value: usize,
    #[serde(default = "FixtureProfileChannel::default_max_value")]
    max_value: usize,
}
impl FixtureProfileChannel {
    const fn default_min_value() -> usize {
        0
    }
    const fn default_max_value() -> usize {
        255
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct FixtureProfileData {
    slug: String,
    label: String,
    channel_count: usize,
    channels: Vec<FixtureProfileChannel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureProfile {
    data: FixtureProfileData,
}
impl FixtureProfile {
    pub async fn load(
        path: impl AsRef<async_std::path::Path>,
    ) -> Result<FixtureProfile, async_std::io::Error> {
        let fixture_profile_contents = async_std::fs::read(path).await?;
        let profile_data: FixtureProfileData = toml::from_slice(&fixture_profile_contents)?;

        // Ensure channel count is correct
        assert_eq!(profile_data.channel_count, profile_data.channels.len());
        Ok(FixtureProfile {data: profile_data})
    }
    fn parameters<'a>(&'a self) -> impl Iterator<Item = FixtureParameter> + 'a {
        self.data.channels.iter().map(|channel| channel.parameter)
    }
    fn has_parameter(&self, parameter: FixtureParameter) -> bool {
        self.parameters().find(|x| *x == parameter).is_some()
    }
    pub fn is_dimmable(&self) -> bool {
        self.has_parameter(FixtureParameter::Dimmer)
    }
    pub fn is_colorable(&self) -> bool {
        self.has_parameter(FixtureParameter::Red)
            && self.has_parameter(FixtureParameter::Green)
            && self.has_parameter(FixtureParameter::Blue)
    }
    pub fn is_positionable(&self) -> bool {
        self.has_parameter(FixtureParameter::Tilt)
            && self.has_parameter(FixtureParameter::Pan)
    }
    fn channel_index(&self, parameter: FixtureParameter) -> Option<usize> {
        self.data.channels.iter().position(|x| x.parameter == parameter)
    }
}

pub async fn load_fixture_profiles(
) -> Result<FxHashMap<String, Arc<FixtureProfile>>, async_std::io::Error> {
    let mut profile_paths = async_std::fs::read_dir("./fixture_profiles").await?;

    let mut fixture_profiles = FxHashMap::default();
    while let Some(entry) = profile_paths.next().await {
        let path = entry?.path();

        let fixture_profile = FixtureProfile::load(path).await?;
        fixture_profiles.insert(fixture_profile.data.slug.clone(), Arc::new(fixture_profile));
    }

    Ok(fixture_profiles)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fixture {
    pub profile: Arc<FixtureProfile>,
    universe: usize,
    start_channel: usize,
    pub group_id: Option<usize>,

    dimmer: f64, // 0.0 - 1.0
    color: Option<palette::LinSrgb<f64>>,
    position: Option<(f64, f64)>, // -1.0 - +1.0
}
impl Fixture {
    pub fn new(
        profile: Arc<FixtureProfile>,
        universe: usize,
        start_channel: usize,
        group_id: Option<usize>,
    ) -> Fixture {
        Fixture {
            profile,
            universe,
            start_channel,
            group_id,
            dimmer: 1.0,
            color: None,
            position: None,
        }
    }
    pub fn set_dimmer(&mut self, dimmer: f64) {
        self.dimmer = dimmer;
    }
    pub fn set_color(
        &mut self,
        color: impl Into<palette::LinSrgb<f64>>,
    ) -> Result<(), &'static str> {
        if self.profile.is_colorable() {
            self.color = Some(color.into());
            Ok(())
        } else {
            Err("Unable to set color. profile does not support it")
        }
    }
    pub fn set_position(&mut self, position: (f64, f64)) -> Result<(), &'static str> {
        if self.profile.is_positionable() {
            self.position = Some(position);
            Ok(())
        } else {
            Err("Unable to set position. profile does not support it")
        }
    }
    pub fn relative_dmx(&self) -> Vec<u8> {
        let mut dmx: Vec<u8> = (0..self.profile.data.channel_count).map(|_| 0).collect();

        if self.profile.is_dimmable() {
            dmx[self
                .profile
                .channel_index(FixtureParameter::Dimmer)
                .unwrap()] = (255 as f64 * self.dimmer) as u8;
        }

        if let Some(color) = self.color {
            let (mut red, mut green, mut blue) = color.into_components();

            // If light doesn't have dimmer control, scale the color values instead
            if !self.profile.is_dimmable() {
                red = red * self.dimmer;
                green = green * self.dimmer;
                blue = blue * self.dimmer;
            }

            dmx[self
                .profile
                .channel_index(FixtureParameter::Red)
                .unwrap()] = (255.0 * red) as u8;
            dmx[self
                .profile
                .channel_index(FixtureParameter::Green)
                .unwrap()] = (255.0 * green) as u8;
            dmx[self
                .profile
                .channel_index(FixtureParameter::Blue)
                .unwrap()] = (255.0 * blue) as u8;
        }

        if let Some(position) = self.position {
            dmx[self
                .profile
                .channel_index(FixtureParameter::Tilt)
                .unwrap()] = (255.0 * ((position.1 + 1.0) / 2.0)) as u8;
            dmx[self
                .profile
                .channel_index(FixtureParameter::Pan)
                .unwrap()] = (255.0 * ((position.0 + 1.0) / 2.0)) as u8;
        }

        dmx
    }
    pub fn absolute_dmx(&self) -> Vec<Option<u8>> {
        (0..(self.start_channel - 1))
            .map(|_| None)
            .chain(self.relative_dmx().into_iter().map(Some))
            .collect()
    }
}
