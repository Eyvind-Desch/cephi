//! Keeps the conversation the model reads from growing without end.
//!
//! Every request re-sends the whole chat, so a long conversation makes each answer slower — on a
//! local model on the CPU, reading the history is most of the wait. Older turns are therefore left
//! out once the chat passes a budget. The cut is always at the start of a user turn, so a tool call
//! never loses its result (every provider rejects that), and the newest turn always goes through,
//! however long it is.

use crate::llm::{ChatMessage, Role};

/// Character budget for a local model. Roughly four characters per token, so about 6000 tokens —
/// a CPU needs seconds to read that much, and small models hold little more.
pub const LOCAL_BUDGET: usize = 24_000;
/// Cloud models read quickly and have room; this only stops a chat that ran away.
pub const CLOUD_BUDGET: usize = 200_000;

/// What a picture costs, whatever its file size: a vision encoder turns it into a fixed number of
/// tokens. Counting its base64 bytes instead would throw out the turn it belongs to.
const IMAGE_COST: usize = 6_000;

/// The end of `history` that fits into `budget`, cut at the start of a user turn.
pub fn within_budget(history: &[ChatMessage], budget: usize) -> &[ChatMessage] {
    let mut kept = None;
    let mut total = 0;
    for (index, message) in history.iter().enumerate().rev() {
        total += cost(message);
        if message.role != Role::User {
            continue;
        }
        // A whole turn is on the table now: take it if it still fits, else stop before it.
        if total > budget && kept.is_some() {
            break;
        }
        kept = Some(index);
    }
    // A history without a user message has no boundary to cut at.
    &history[kept.unwrap_or(0)..]
}

/// Roughly what one message costs the model to read.
fn cost(message: &ChatMessage) -> usize {
    let calls: usize = message
        .tool_calls
        .iter()
        .map(|call| call.name.len() + call.arguments.to_string().len())
        .sum();
    // `native` is replayed verbatim (e.g. Claude thinking blocks), so it is part of the payload.
    let native = message.native.as_ref().map_or(0, |turn| turn.content.to_string().len());
    message.content.len() + message.images.len() * IMAGE_COST + calls + native
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ToolCall;

    fn call(name: &str) -> ToolCall {
        ToolCall {
            id: name.into(),
            name: name.into(),
            arguments: serde_json::json!({}),
        }
    }

    /// One user question, the tool it triggered and the answer.
    fn turn(question: &str, filler: usize) -> Vec<ChatMessage> {
        vec![
            ChatMessage::user(question),
            ChatMessage::assistant_tool_calls(vec![call("clock")]),
            ChatMessage::tool_result(&call("clock"), "x".repeat(filler), true),
            ChatMessage::assistant("answer"),
        ]
    }

    #[test]
    fn a_short_chat_goes_through_untouched() {
        let history = [turn("one", 10), turn("two", 10)].concat();
        assert_eq!(within_budget(&history, 1000).len(), history.len());
    }

    #[test]
    fn older_turns_are_left_out_whole() {
        let history = [turn("one", 500), turn("two", 500), turn("three", 500)].concat();
        let kept = within_budget(&history, 1200);
        assert_eq!(kept.len(), 8, "the last two turns fit, the first does not");
        assert_eq!(kept[0].content, "two");
    }

    #[test]
    fn a_cut_never_separates_a_tool_result_from_its_call() {
        let history = [turn("one", 900), turn("two", 900)].concat();
        for budget in 0..2500 {
            let kept = within_budget(&history, budget);
            assert_eq!(kept[0].role, Role::User, "budget {budget} cut inside a turn");
        }
    }

    #[test]
    fn the_newest_turn_is_sent_however_long_it_is() {
        let history = turn("one", 100_000);
        assert_eq!(within_budget(&history, 10).len(), history.len());
    }

    #[test]
    fn pictures_count_even_though_they_are_short_strings() {
        let history = vec![
            ChatMessage::user("older question"),
            ChatMessage::assistant("older answer"),
            ChatMessage::user_with_images("what is on it?", vec!["png".into()]),
        ];
        assert_eq!(within_budget(&history, IMAGE_COST / 2).len(), 1);
        assert_eq!(within_budget(&history, IMAGE_COST * 2).len(), 3);
    }
}
