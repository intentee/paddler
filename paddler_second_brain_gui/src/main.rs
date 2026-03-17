use iced::Center;
use iced::widget::Column;
use iced::widget::button;
use iced::widget::column;

fn main() -> iced::Result {
    iced::run(SecondBrain::update, SecondBrain::view)
}

#[derive(Default)]
struct SecondBrain;

#[derive(Debug, Clone, Copy)]
enum Message {
    ButtonPressed,
}

impl SecondBrain {
    fn update(&mut self, message: Message) {
        match message {
            Message::ButtonPressed => {}
        }
    }

    fn view<'view>(&'view self) -> Column<'view, Message> {
        column![button("Hello from Paddler").on_press(Message::ButtonPressed),]
            .padding(20)
            .align_x(Center)
    }
}
