//! The title page of a screenplay, teleplay or stage script.
//!
//! One description, read by every writer, so the cover reads the same in each
//! format. It follows the trade convention rather than a book's: the title in
//! capitals, centred about a third of the way down; "Written by" and the
//! author's name beneath it; and the draft line low on the page at the left.
//! All of it is in the script's monospaced face at the script's own size, with
//! none of a book's subtitle or imprint.
//!
//! The lines carry ordinary `ElementStyle`s (space above, alignment, size), so
//! a flowing format reads them as paragraphs and the PDF places them from the
//! same numbers.

use super::*;

/// What a line of the cover is, so a format can name a style for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TitleRole {
    Title,
    Credit,
    Footer,
}

pub(crate) struct ScriptTitleLine {
    pub(crate) role: TitleRole,
    pub(crate) text: String,
    pub(crate) style: ElementStyle,
}

/// Type size of the whole cover, in points, on US Letter.
const COVER_SIZE_PT: f32 = 12.0;
/// Blank lines between the title and "Written by", and before the draft line
/// is pushed to the foot of the page.
const BLANK_LINES_BEFORE_CREDIT: f32 = 2.0;
/// Line height the flowing formats are asked for.
const COVER_LEADING: f32 = 1.0;
/// A flowing format cannot know a word processor's exact line height, so the
/// draft line stops this many lines short of the margin rather than risk
/// spilling onto a second page.
const FOOTER_SLACK_LINES: f32 = 3.0;

/// Break `text` into lines no wider than `max_chars`.
fn wrap_title(text: &str, max_chars: usize) -> Vec<String> {
    crate::pdf::wrap_plain(text, max_chars)
}

/// The cover's lines, top to bottom, for this compile.
pub(crate) fn script_cover_lines(project: &Project, settings: &ExportSettings) -> Vec<ScriptTitleLine> {
    let scale = script_scale(settings);
    let size = COVER_SIZE_PT * scale;
    let (page_w, page_h) = paper_pt(settings);
    let margins = page_margins(project, settings);
    let content_w = (page_w - margins.left - margins.right).max(24.0);
    let content_h = (page_h - margins.top - margins.bottom).max(24.0);
    // Courier advances 0.6 em; wrapping by the real text width keeps a long
    // title inside the margins on A5 as well as on Letter.
    let max_chars = ((content_w / (size * 0.6)).floor() as usize).max(12);

    let style = |align: Alignment, before: f32| ElementStyle {
        font: String::new(),
        size,
        leading: COVER_LEADING,
        before,
        align,
        ..ElementStyle::default()
    };
    let blank = size * COVER_LEADING * BLANK_LINES_BEFORE_CREDIT;
    let mut lines = Vec::new();
    let mut used = 0.0f32;
    let mut push = |role: TitleRole, text: String, style: ElementStyle, used: &mut f32| {
        *used += style.before + style.size * style.leading;
        lines.push(ScriptTitleLine { role, text, style });
    };

    let drop = title_drop_for(project, settings, size);
    for (index, text) in wrap_title(&project.title.trim().to_uppercase(), max_chars).into_iter().enumerate() {
        let before = if index == 0 { drop } else { 0.0 };
        let mut line_style = style(Alignment::Center, before);
        line_style.uppercase = true;
        push(TitleRole::Title, text, line_style, &mut used);
    }
    if !project.author.trim().is_empty() {
        push(TitleRole::Credit, "Written by".into(), style(Alignment::Center, blank), &mut used);
        for text in wrap_title(project.author.trim(), max_chars) {
            push(TitleRole::Credit, text, style(Alignment::Center, 0.0), &mut used);
        }
    }
    let footer_height = size * COVER_LEADING;
    let gap = (content_h - used - footer_height - FOOTER_SLACK_LINES * footer_height).max(blank);
    let today = chrono::Local::now().format("%d %B %Y");
    push(
        TitleRole::Footer,
        format!("Draft · {today}"),
        style(Alignment::Left, gap),
        &mut used,
    );
    lines
}
