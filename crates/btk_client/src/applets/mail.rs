use super::Applet;

#[derive(Default)]
pub struct MailApplet;
impl Applet for MailApplet {
    fn name(&self) -> &str {
        "Mail"
    }
}
