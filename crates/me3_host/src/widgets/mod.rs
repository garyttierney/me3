

use futures::{
    channel::mpsc::{Receiver, Sender},
};
use me3_framework::overlay::{
    Align2, Context, ScrollArea, TextEdit, TextStyle, Ui, Vec2, Visuals, Window,
};
use ringbuffer::{AllocRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};

pub struct Console {
    lines: AllocRingBuffer<String>,
    input: String,

    /// Channel that commands are sent to for execution.
    command_tx: Sender<String>,

    /// Channel that command output is received from.
    command_output_rx: Receiver<String>,
}

impl Console {
    pub fn new(command_tx: Sender<String>, command_output_rx: Receiver<String>) -> Self {
        let mut lines = AllocRingBuffer::with_capacity(1024);
        lines.fill_default();

        Console {
            lines,
            input: String::default(),
            command_tx,
            command_output_rx,
        }
    }

    pub fn render(&mut self, ctx: &Context) {
        let old_visuals = Visuals::dark();
        let mut console_visuals = old_visuals.clone();

        console_visuals.widgets.noninteractive.bg_fill = old_visuals
            .widgets
            .noninteractive
            .bg_fill
            .linear_multiply(0.25);

        ctx.set_visuals(console_visuals);

        Window::new("Console")
            .resizable(true)
            .collapsible(false)
            .default_width(f32::INFINITY)
            .min_height(200.0)
            .anchor(Align2::LEFT_TOP, Vec2::ZERO)
            .title_bar(false)
            .show(ctx, |ui| self.render_ui(ui));

        ctx.set_visuals(old_visuals);
    }

    fn render_ui(&mut self, ui: &mut Ui) {
        while let Ok(Some(line)) = self.command_output_rx.try_next() {
            self.lines.push(line);
        }

        let text_style = TextStyle::Monospace;
        let row_height = ui.text_style_height(&text_style);

        ScrollArea::vertical()
            .stick_to_bottom(true)
            .always_show_scroll(true)
            .auto_shrink([false, false])
            .max_height(ui.available_height() - row_height * 2.0)
            .show_rows(ui, row_height, self.lines.len(), |ui, row_range| {
                for row in row_range {
                    let line = self.lines.get(row as isize);
                    ui.label(line.expect("invalid offset for line"));
                }
            });

        let input_response = ui.add(
            TextEdit::singleline(&mut self.input)
                .desired_width(f32::INFINITY)
                .hint_text("Enter a command"),
        );

        if input_response.lost_focus() {
            let _ = self.command_tx.try_send(self.input.clone());
        }
    }
}
