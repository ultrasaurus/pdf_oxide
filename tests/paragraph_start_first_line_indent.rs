//! Paragraph starts marked only by a first-line indent, with no extra space
//! between paragraphs: the line after a paragraph's short last line starts
//! to the right of the column's left edge, and the line after it returns to
//! that edge.
//!
//! All PDFs are hand-built, untagged, single-page documents.

// Only `obj` and `stream` are used here, not the page builder.
#[allow(dead_code)]
mod common;

use common::{obj, stream};
use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::PdfDocument;

/// One untagged page of Helvetica 12pt text. Each line is (text, x, y).
fn lines_pdf(lines: &[(&str, i32, i32)]) -> Vec<u8> {
    let mut content = String::from("BT /F1 12 Tf\n");
    for (txt, x, y) in lines {
        content.push_str(&format!("1 0 0 1 {x} {y} Tm ({txt}) Tj\n"));
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

/// Paragraphs whose first line is shorter (it is indented) and whose last
/// line stops well short of the right margin.
const PARA_A: [&str; 4] = [
    "When you asked me to speak about women and fiction",
    "I sat down on the banks of a river and began to wonder what",
    "the words meant. They might mean simply a few remarks about",
    "Fanny Burney and Jane Austen.",
];
const PARA_B: [&str; 4] = [
    "Here then was I sitting on the banks of a river a week",
    "or two ago in fine October weather, lost in thought. That",
    "collar I have spoken of, women and fiction, the need of",
    "coming to some conclusion.",
];

/// `paras` from the top of the page at `pitch` points per line, no extra
/// space between paragraphs, each first line indented by `indent` points.
fn indented(paras: &[&[&'static str]], pitch: i32, indent: i32) -> Vec<(&'static str, i32, i32)> {
    let mut y = 700;
    let mut out = Vec::new();
    for para in paras {
        for (i, line) in para.iter().enumerate() {
            out.push((*line, if i == 0 { 72 + indent } else { 72 }, y));
            y -= pitch;
        }
    }
    out
}

fn options() -> ConversionOptions {
    ConversionOptions::default()
}

/// Paragraphs of a blank-line separated text.
fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect()
}

fn assert_two_paragraphs(lines: &[(&str, i32, i32)]) {
    let doc = PdfDocument::from_bytes(lines_pdf(lines)).unwrap();
    let md = paragraphs(&doc.to_markdown(0, &options()).unwrap());
    assert_eq!(md.len(), 2, "markdown: {md:#?}");
    assert!(md[1].starts_with("Here then"), "markdown: {md:#?}");
    let text = paragraphs(&doc.to_plain_text(0, &options()).unwrap());
    assert_eq!(text.len(), 2, "plain text: {text:#?}");
    let html = doc.to_html(0, &options()).unwrap();
    assert_eq!(html.matches("<p>").count(), 2, "html: {html}");
}

#[test]
fn double_spaced_indented_paragraphs_split() {
    assert_two_paragraphs(&indented(&[&PARA_A, &PARA_B], 24, 36));
}

#[test]
fn single_spaced_indented_paragraphs_split() {
    assert_two_paragraphs(&indented(&[&PARA_A, &PARA_B], 14, 24));
}

#[test]
fn hanging_indent_continuation_stays_with_its_entry() {
    // Reference-list style: each entry's first line runs to the margin and
    // its continuation is indented. The continuation follows a full line,
    // so it does not start a paragraph.
    let lines = [
        ("Snyder, C. R. (1989). Reality negotiation: From excuses to", 72, 700),
        ("hope and beyond.", 108, 686),
        ("Stotland, E. (1969). The psychology of hope. San Francisco:", 72, 672),
        ("Jossey-Bass.", 108, 658),
    ];
    let doc = PdfDocument::from_bytes(lines_pdf(&lines)).unwrap();
    let md = paragraphs(&doc.to_markdown(0, &options()).unwrap());
    assert!(
        md.iter()
            .all(|p| !p.starts_with("hope and beyond") && !p.starts_with("Jossey")),
        "markdown: {md:#?}"
    );
}
