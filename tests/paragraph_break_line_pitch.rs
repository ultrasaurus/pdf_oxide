//! Paragraph breaks on pages whose lines are set further apart than 1.5×
//! the font size — double-spaced text, or a scanned page's text layer whose
//! reported font size is small for its line spacing.
//!
//! A gap is a paragraph break when it is larger than the page's own line
//! pitch, not when it passes a fixed multiple of the font size: a wrapped
//! line of double-spaced prose must stay in its paragraph, while an extra
//! gap between paragraphs still separates them.
//!
//! All PDFs are hand-built, untagged, single-page documents.

// Only `obj` and `stream` are used here, not the page builder.
#[allow(dead_code)]
mod common;

use common::{obj, stream};
use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::PdfDocument;

/// One untagged page of Helvetica `size`pt text. Each line is (text, x, y).
fn lines_pdf(size: u32, lines: &[(&str, i32, i32)]) -> Vec<u8> {
    let sized: Vec<_> = lines.iter().map(|&(t, x, y)| (size, t, x, y)).collect();
    sized_lines_pdf(&sized)
}

/// One untagged page of Helvetica text. Each line is (size, text, x, y).
fn sized_lines_pdf(lines: &[(u32, &str, i32, i32)]) -> Vec<u8> {
    let mut content = String::from("BT\n");
    for (size, txt, x, y) in lines {
        content.push_str(&format!("/F1 {size} Tf 1 0 0 1 {x} {y} Tm ({txt}) Tj\n"));
    }
    content.push_str("ET\n");

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    stream(&mut buf, &mut off, 4, content.as_bytes());
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for o in &off[1..] {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n").as_bytes(),
    );
    buf
}

/// Wrapped prose lines of about the same width, as a justified or ragged
/// column produces them.
const PARA_A: [&str; 5] = [
    "When you asked me to speak about women and fiction I sat",
    "down on the banks of a river and began to wonder what the",
    "words meant. They might mean simply a few remarks about",
    "Fanny Burney; a few more about Jane Austen; a tribute to",
    "the Brontes and a sketch of Haworth Parsonage under snow.",
];
const PARA_B: [&str; 4] = [
    "Here then was I sitting on the banks of a river a week or",
    "two ago in fine October weather, lost in thought. That",
    "collar I have spoken of, women and fiction, the need of",
    "coming to some conclusion on a subject that raises all.",
];

/// `paras` laid out from the top of the page at `pitch` points per line,
/// with `extra` more points between paragraphs.
fn layout(paras: &[&[&'static str]], pitch: i32, extra: i32) -> Vec<(&'static str, i32, i32)> {
    let mut y = 700;
    let mut out = Vec::new();
    for para in paras {
        for line in *para {
            out.push((*line, 72, y));
            y -= pitch;
        }
        y -= extra;
    }
    out
}

fn markdown(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    doc.to_markdown(0, &ConversionOptions::default()).unwrap()
}

fn html(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    doc.to_html(0, &ConversionOptions::default()).unwrap()
}

fn plain_text(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    doc.to_plain_text(0, &ConversionOptions::default()).unwrap()
}

/// Paragraphs of a blank-line separated text.
fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect()
}

#[test]
fn double_spaced_lines_stay_one_paragraph() {
    // 12pt on a 24pt pitch: every gap is 2× the font size.
    let pdf = || lines_pdf(12, &layout(&[&PARA_A], 24, 0));

    let md = paragraphs(&markdown(pdf()));
    assert_eq!(md.len(), 1, "markdown: {md:#?}");
    let text = paragraphs(&plain_text(pdf()));
    assert_eq!(text.len(), 1, "plain text: {text:#?}");
    let h = html(pdf());
    assert_eq!(h.matches("<p>").count(), 1, "html: {h}");
}

#[test]
fn small_font_size_on_wider_pitch_lines_stay_one_paragraph() {
    // As in a scanned page's text layer: 8pt text on a pitch alternating
    // 12 and 13pt (about 1.56×).
    let lines: Vec<_> = PARA_A
        .iter()
        .enumerate()
        .map(|(i, l)| (*l, 72, 700 - (i as i32) * 25 / 2))
        .collect();
    let md = paragraphs(&markdown(lines_pdf(8, &lines)));
    assert_eq!(md.len(), 1, "markdown: {md:#?}");
}

#[test]
fn double_spaced_paragraphs_with_extra_gap_split() {
    // 24pt pitch, one more line's worth of space between paragraphs.
    let pdf = || lines_pdf(12, &layout(&[&PARA_A, &PARA_B], 24, 24));

    let md = paragraphs(&markdown(pdf()));
    assert_eq!(md.len(), 2, "markdown: {md:#?}");
    assert!(md[1].starts_with("Here then"), "markdown: {md:#?}");
    let text = paragraphs(&plain_text(pdf()));
    assert_eq!(text.len(), 2, "plain text: {text:#?}");
    let h = html(pdf());
    assert_eq!(h.matches("<p>").count(), 2, "html: {h}");
}

#[test]
fn title_over_double_spaced_text_stays_separate() {
    // A 20pt title 40pt above 12pt body text on a 24pt pitch. The body's
    // measured ratio (2.5×) would hide the 40pt gap when applied to the
    // title's size (2.5 × 20 = 50), so between different sizes the fixed
    // ratio (1.5 × 20 = 30) still decides.
    let mut lines = vec![(20, "A Room of One's Own", 72, 740)];
    lines.extend(
        layout(&[&PARA_A], 24, 0)
            .into_iter()
            .map(|(t, x, y)| (12, t, x, y)),
    );
    let text = paragraphs(&plain_text(sized_lines_pdf(&lines)));
    assert_eq!(text.len(), 2, "plain text: {text:#?}");
    assert_eq!(text[0], "A Room of One's Own", "plain text: {text:#?}");
}

#[test]
fn single_spaced_paragraphs_with_extra_gap_split() {
    // The ordinary case the fixed ratio was built for: 12pt on a 14pt
    // pitch, 22pt between paragraphs (over 1.5 × 12 = 18).
    let md = paragraphs(&markdown(lines_pdf(12, &layout(&[&PARA_A, &PARA_B], 14, 8))));
    assert_eq!(md.len(), 2, "markdown: {md:#?}");
}

#[test]
fn short_lines_spaced_apart_stay_separate() {
    // Short lines (none reaching a common right margin) 30pt apart: there
    // is no wrapped prose to measure a pitch from, so each stays its own
    // paragraph, as before.
    let lines = [
        ("Name", 72, 700),
        ("Address", 72, 670),
        ("Telephone", 72, 640),
        ("Email", 72, 610),
        ("Notes", 72, 580),
    ];
    let md = paragraphs(&markdown(lines_pdf(12, &lines)));
    assert_eq!(md.len(), 5, "markdown: {md:#?}");
}
