use strum::EnumString;

#[derive(strum::Display, strum::EnumIter, Debug, Eq, PartialEq, EnumString, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum Event {
    ProjectPeek,
    ProjectSwitch,
    ProjectUpdate,
    PluginEnable,
    PluginDisable,
}
