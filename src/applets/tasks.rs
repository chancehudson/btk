use super::Applet;

#[derive(Default)]
pub struct TasksApplet {}

impl Applet for TasksApplet {
    fn name(&self) -> &str {
        "Tasks"
    }
}
