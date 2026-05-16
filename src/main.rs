use std::borrow::Cow;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, Reedline, Signal};

fn main() {
    fletch::run_repl(|mut ctx| {
        let mut line_editor = Reedline::create().with_validator(ctx.validator());

        loop {
            let sig = line_editor.read_line(&LangPrompt);
            match sig {
                Ok(Signal::Success(buffer)) => match buffer.trim() {
                    ".env" => ctx.print_env(),
                    ".exit" => break,
                    _ => ctx.eval(&buffer),
                },
                Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => break,
                x => println!("Event: {:?}", x),
            }
        }
    });
}

struct LangPrompt;

impl Prompt for LangPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("… ") // same visual width as "〉 "
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
}
