use super::Applet;

#[derive(Default)]
pub struct SettingsApplet {}

impl Applet for SettingsApplet {
    fn name(&self) -> &str {
        "Settings"
    }
}
