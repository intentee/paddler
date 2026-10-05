use futures::StreamExt as _;
use futures::channel::mpsc;
use futures::executor::block_on;
use iced_test::Emulator;
use iced_test::Instruction;
use iced_test::core::Size;
use iced_test::core::mouse::Button;
use iced_test::emulator::Event as EmulatorEvent;
use iced_test::emulator::Mode;
use iced_test::instruction::Expectation;
use iced_test::instruction::Interaction;
use iced_test::instruction::Keyboard;
use iced_test::instruction::Mouse;
use iced_test::instruction::Target;
use iced_test::program::Program;

use crate::gui_test_error::GuiTestError;

pub struct EmulatedApp<TProgram: Program + 'static> {
    emulator: Emulator<TProgram>,
    events: mpsc::Receiver<EmulatorEvent<TProgram>>,
    program: TProgram,
}

impl<TProgram: Program + 'static> EmulatedApp<TProgram> {
    pub fn boot(program: TProgram, window_size: Size) -> Result<Self, GuiTestError> {
        let (event_tx, events) = mpsc::channel(0);
        let emulator = Emulator::new(event_tx, &program, Mode::Immediate, window_size);
        let mut emulated_app = Self {
            emulator,
            events,
            program,
        };

        emulated_app.wait_until_ready().map(|()| emulated_app)
    }

    pub fn click(&mut self, label: &str) -> Result<(), GuiTestError> {
        self.run(Instruction::Interact(Interaction::Mouse(Mouse::Click {
            button: Button::Left,
            target: Some(Target::Text(label.to_owned())),
        })))
    }

    pub fn expect_text(&mut self, text: &str) -> Result<(), GuiTestError> {
        self.run(Instruction::Expect(Expectation::Text(text.to_owned())))
    }

    pub fn perform_next_action(&mut self) -> Result<(), GuiTestError> {
        match block_on(self.events.next()) {
            Some(EmulatorEvent::Action(action)) => {
                self.emulator.perform(&self.program, action);

                Ok(())
            }
            Some(EmulatorEvent::Failed(instruction)) => {
                Err(GuiTestError::InstructionFailed { instruction })
            }
            Some(EmulatorEvent::Ready) => Err(GuiTestError::EmulatorReadyWithoutAction),
            None => Err(GuiTestError::EmulatorStopped),
        }
    }

    pub fn typewrite(&mut self, text: &str) -> Result<(), GuiTestError> {
        self.run(Instruction::Interact(Interaction::Keyboard(
            Keyboard::Typewrite(text.to_owned()),
        )))
    }

    pub fn update(&mut self, message: TProgram::Message) {
        self.emulator.update(&self.program, message);
    }

    fn run(&mut self, instruction: Instruction) -> Result<(), GuiTestError> {
        self.emulator.run(&self.program, instruction);

        self.wait_until_ready()
    }

    fn wait_until_ready(&mut self) -> Result<(), GuiTestError> {
        loop {
            match block_on(self.events.next()) {
                Some(EmulatorEvent::Action(action)) => {
                    self.emulator.perform(&self.program, action);
                }
                Some(EmulatorEvent::Failed(instruction)) => {
                    return Err(GuiTestError::InstructionFailed { instruction });
                }
                Some(EmulatorEvent::Ready) => return Ok(()),
                None => return Err(GuiTestError::EmulatorStopped),
            }
        }
    }
}
