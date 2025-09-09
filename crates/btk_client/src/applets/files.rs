use super::Applet;

#[derive(Default)]
pub struct FilesApplet {}

impl Applet for FilesApplet {
    fn name(&self) -> &str {
        "Files"
    }
}
