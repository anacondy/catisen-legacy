use font8x8::{UnicodeFonts, BASIC_FONTS};
use image::{Rgba, RgbaImage};
use scraper::{Html, Selector};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;



pub const VISUAL_SNAPSHOT_PREFIX: &str = "__VISUAL_SNAPSHOT__::";
pub const VISUAL_SPIKE_MANIFEST_PREFIX: &str = "__VISUAL_SPIKE_MANIFEST__::";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualEngine {
    SnapshotBridge,
    ServoSpike,
}

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
struct DomNodeExcerpt {
    tag: String,
    text: String,
    char_count: usize,
    attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
struct HtmlParseDiagnostic {
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct DomParseOutput {
    title: String,
    element_count: usize,
    text_node_count: usize,
    excerpts: Vec<DomNodeExcerpt>,
    parse_diagnostics: Vec<HtmlParseDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
struct CssStylesheetPayload {
    origin: String, // "inline", "internal", or "external"
    content: String,
    estimated_rules: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ComputedStyleSnapshot {
    node_tag: String,
    classes: Vec<String>,
    resolved_styles: usize,
}

#[derive(Debug, Clone, Serialize)]
struct StyleResolveOutput {
    style_block_count: usize,
    style_rule_estimate: usize,
    inline_style_count: usize,
    inline_color_decls: usize,
    inline_background_decls: usize,
    external_stylesheet_links: usize,
    extracted_stylesheets: Vec<CssStylesheetPayload>,
    computed_node_snapshots: Vec<ComputedStyleSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct BoxModelMetrics {
    margin_top: u32,
    margin_bottom: u32,
    padding: u32,
    border_width: u32,
}

#[derive(Debug, Clone, Serialize)]
struct LayoutBlockOutput {
    tag: String,
    text: String,
    display_type: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    font_scale: u32,
    box_model: BoxModelMetrics,
    attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
struct LayoutOutput {
    viewport_width: u32,
    viewport_height: u32,
    content_height: u32,
    truncated: bool,
    block_count: usize,
    blocks: Vec<LayoutBlockOutput>,
}

#[derive(Debug, Clone, Serialize)]
struct StageTimings {
    dom_parse_ms: u128,
    style_resolve_ms: u128,
    layout_ms: u128,
    paint_ms: u128,
    total_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
struct SpikeManifest {
    timestamp_ms: u128,
    url: String,
    html_bytes: usize,
    injected_script_bytes: usize,
    render_backend: String,
    stage_timings: StageTimings,
    dom_parse: DomParseOutput,
    style_resolve: StyleResolveOutput,
    layout: LayoutOutput,
    output_frame_png: String,
    note: String,
}

fn selected_visual_engine() -> VisualEngine {
    match std::env::var("CATISEN_VISUAL_ENGINE") {
        Ok(raw) => match raw.trim().to_lowercase().as_str() {
            "snapshot" | "bridge" => VisualEngine::SnapshotBridge,
            "servo-spike" | "spike" | "servo" => VisualEngine::ServoSpike,
            _ => VisualEngine::ServoSpike,
        },
        Err(_) => VisualEngine::ServoSpike,
    }
}

fn normalize_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn selector(query: &str) -> Option<Selector> {
    Selector::parse(query).ok()
}

fn parse_dom_stage(document: &Html) -> DomParseOutput {
    let title = selector("title")
        .and_then(|sel| document.select(&sel).next())
        .map(|el| normalize_whitespace(&el.text().collect::<Vec<_>>().join(" ")))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled Page".to_string());

    let mut excerpts: Vec<DomNodeExcerpt> = Vec::new();
    let mut text_node_count = 0usize;

    if let Some(all_sel) = selector("*") {
        for element in document.select(&all_sel) {
            let text = normalize_whitespace(&element.text().collect::<Vec<_>>().join(" "));
            if !text.is_empty() {
                text_node_count += 1;
            }
        }
    }

    let tag_order = ["h1", "h2", "h3", "p", "li", "a", "code", "pre", "form", "input", "button", "video", "audio", "iframe", "nav", "article", "img"];
    for tag in tag_order {
        if excerpts.len() >= 160 {
            break;
        }
        if let Some(sel) = selector(tag) {
            for element in document.select(&sel) {
                if excerpts.len() >= 160 {
                    break;
                }
                
                let mut attributes = HashMap::new();
                for (k, v) in element.value().attrs() {
                    attributes.insert(k.to_string(), v.to_string());
                }

                if tag == "img" || tag == "video" || tag == "audio" || tag == "iframe" {
                    let mut fallback_text = format!("[{}:...]", tag.to_uppercase());
                    if let Some(src) = element.value().attr("src") {
                        // Skip base64 encoded data URIs - they clutter the layout
                        if src.starts_with("data:") {
                            fallback_text = format!("[{}]", tag.to_uppercase());
                        } else {
                            fallback_text = format!("[{}: {}]", tag.to_uppercase(), truncate_chars(src, 80));
                        }
                    }

                    excerpts.push(DomNodeExcerpt {
                        tag: tag.to_string(),
                        char_count: fallback_text.chars().count(),
                        text: fallback_text,
                        attributes,
                    });
                    continue;
                }
                
                let text = normalize_whitespace(&element.text().collect::<Vec<_>>().join(" "));
                if text.is_empty() && tag != "input" && tag != "form" {
                    continue;
                }
                excerpts.push(DomNodeExcerpt {
                    tag: tag.to_string(),
                    char_count: text.chars().count(),
                    text: truncate_chars(&text, 220),
                    attributes,
                });
            }
        }
    }

    if excerpts.is_empty() {
        let fallback_text = normalize_whitespace(&document.root_element().text().collect::<Vec<_>>().join(" "));
        if !fallback_text.is_empty() {
            excerpts.push(DomNodeExcerpt {
                tag: "body".to_string(),
                char_count: fallback_text.chars().count(),
                text: truncate_chars(&fallback_text, 240),
                attributes: HashMap::new(),
            });
        }
    }

    let element_count = selector("*")
        .map(|sel| document.select(&sel).count())
        .unwrap_or(0);

    let parse_diagnostics = document
        .errors
        .iter()
        .map(|err| HtmlParseDiagnostic {
            message: err.to_string(),
        })
        .collect();

    DomParseOutput {
        title,
        element_count,
        text_node_count,
        excerpts,
        parse_diagnostics,
    }
}

fn parse_style_stage(document: &Html) -> StyleResolveOutput {
    let style_block_count = selector("style")
        .map(|sel| document.select(&sel).count())
        .unwrap_or(0);

    let style_rule_estimate = selector("style")
        .map(|sel| {
            document
                .select(&sel)
                .map(|el| el.text().collect::<Vec<_>>().join(" ").matches('{').count())
                .sum()
        })
        .unwrap_or(0usize);

    let external_stylesheet_links = selector("link[rel=\"stylesheet\"]")
        .map(|sel| document.select(&sel).count())
        .unwrap_or(0);

    let mut inline_style_count = 0usize;
    let mut inline_color_decls = 0usize;
    let mut inline_background_decls = 0usize;

    if let Some(all_sel) = selector("*") {
        for element in document.select(&all_sel) {
            if let Some(style_attr) = element.value().attr("style") {
                inline_style_count += 1;
                let lower = style_attr.to_lowercase();
                if lower.contains("color:") {
                    inline_color_decls += 1;
                }
                if lower.contains("background") {
                    inline_background_decls += 1;
                }
            }
        }
    }

    let mut extracted_stylesheets = Vec::new();
    if let Some(sel) = selector("style") {
        for element in document.select(&sel) {
            let css_text = element.text().collect::<Vec<_>>().join(" ");
            let rule_count = css_text.matches('{').count();
            extracted_stylesheets.push(CssStylesheetPayload {
                origin: "internal".to_string(),
                content: truncate_chars(&css_text, 1000), // Prevent massive payload strings
                estimated_rules: rule_count,
            });
        }
    }
    
    // Stub computed stylesheets snapshot simulation across tags mapping to rules
    let mut computed_node_snapshots = Vec::new();
    let sample_tags = ["body", "h1", "p", "div", "a"];
    for tag in sample_tags {
        if let Some(sel) = selector(tag) {
            for element in document.select(&sel).take(5) {
                computed_node_snapshots.push(ComputedStyleSnapshot {
                    node_tag: tag.to_string(),
                    classes: element.value().classes().map(|s| s.to_string()).collect(),
                    resolved_styles: 1 + inline_style_count.min(3), // Simulated computation hit
                });
            }
        }
    }

    StyleResolveOutput {
        style_block_count,
        style_rule_estimate,
        inline_style_count,
        inline_color_decls,
        inline_background_decls,
        external_stylesheet_links,
        extracted_stylesheets,
        computed_node_snapshots,
    }
}

fn layout_stage(dom: &DomParseOutput, viewport_width: u32, viewport_height: u32) -> LayoutOutput {
    let margin = 24u32;
    let content_width = viewport_width.saturating_sub(margin * 2).max(120);
    let mut current_y = 72u32;
    let mut current_x = margin;
    let mut line_height_max = 0u32;
    let max_content_height = viewport_height.saturating_mul(3);
    let mut truncated = false;
    let mut blocks: Vec<LayoutBlockOutput> = Vec::new();

    for node in &dom.excerpts {
        let (font_scale, base_height, display_type, margin_m, padding) = match node.tag.as_str() {
            "h1" => (2u32, 34u32, "block", 24u32, 4u32),
            "h2" => (2u32, 30u32, "block", 20u32, 4u32),
            "h3" => (2u32, 28u32, "block", 18u32, 2u32),
            "pre" | "code" => (1u32, 26u32, "block", 12u32, 8u32),
            "li" => (1u32, 22u32, "list-item", 8u32, 2u32),
            "a" | "span" => (1u32, 24u32, "inline", 8u32, 2u32),
            "img" => (1u32, 160u32, "inline", 12u32, 0u32), // Taller element width/height for image boundaries
            "video" | "iframe" => (1u32, 280u32, "block", 24u32, 0u32),
            "audio" => (1u32, 48u32, "block", 12u32, 0u32),
            _ => (1u32, 24u32, "block", 16u32, 0u32),
        };

        let text_len = node.text.chars().count().max(1);
        
        let block_width = if display_type == "inline" {
            let native_width = if node.tag == "img" { 240u32 } else { ((text_len as u32) * ((8 * font_scale) + 2)).saturating_add(padding * 2) };
            native_width.clamp(30, content_width)
        } else {
            content_width
        };

        // Layout wrapping rules (Flex-style Wrap)
        if display_type == "block" || display_type == "list-item" || (current_x > margin && current_x + block_width > margin + content_width) {
            // New line jump
            if current_x > margin {
                current_y = current_y.saturating_add(line_height_max).saturating_add(margin_m);
            } else {
                current_y = current_y.saturating_add(margin_m); // Safe push if we were already on a fresh line
            }
            current_x = margin;
            line_height_max = 0;
        }

        let chars_per_line = (block_width / ((8 * font_scale) + 2)).max(12) as usize;
        let line_count = if node.tag == "img" { 1 } else { ((text_len + chars_per_line - 1) / chars_per_line).clamp(1, 8) as u32 };
        let line_height = (8 * font_scale) + 5;
        let block_height = (line_count * line_height + (padding * 2)).max(base_height);

        if current_y.saturating_add(block_height).saturating_add(margin_m) >= max_content_height {
            truncated = true;
            break;
        }

        blocks.push(LayoutBlockOutput {
            tag: node.tag.clone(),
            text: node.text.clone(),
            display_type: display_type.to_string(),
            x: current_x,
            y: current_y,
            width: block_width,
            height: block_height,
            font_scale,
            box_model: BoxModelMetrics {
                margin_top: margin_m,
                margin_bottom: margin_m,
                padding,
                border_width: if block_height > 0 { 1 } else { 0 },
            },
            attributes: node.attributes.clone(),
        });

        line_height_max = line_height_max.max(block_height);

        if display_type == "inline" {
            current_x = current_x.saturating_add(block_width).saturating_add(margin_m);
        } else {
            current_y = current_y.saturating_add(block_height).saturating_add(margin_m);
            current_x = margin;
            line_height_max = 0;
        }
    }

    if blocks.is_empty() {
        blocks.push(LayoutBlockOutput {
            tag: "body".to_string(),
            text: "No parseable text blocks found. Page may be JS-driven.".to_string(),
            display_type: "block".to_string(),
            x: margin,
            y: current_y,
            width: content_width,
            height: 36,
            font_scale: 1,
            box_model: BoxModelMetrics {
                margin_top: 0, margin_bottom: 0, padding: 8, border_width: 0
            },
            attributes: HashMap::new(),
        });
        current_y = current_y.saturating_add(44);
    }

    LayoutOutput {
        viewport_width,
        viewport_height,
        content_height: current_y.saturating_add(line_height_max).saturating_add(margin),
        truncated,
        block_count: blocks.len(),
        blocks,
    }
}

fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    let max_x = x.saturating_add(w).min(img.width());
    let max_y = y.saturating_add(h).min(img.height());

    for yy in y..max_y {
        for xx in x..max_x {
            img.put_pixel(xx, yy, color);
        }
    }
}

fn draw_rect_outline(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    if w == 0 || h == 0 {
        return;
    }

    let right = x.saturating_add(w.saturating_sub(1)).min(img.width().saturating_sub(1));
    let bottom = y.saturating_add(h.saturating_sub(1)).min(img.height().saturating_sub(1));

    for xx in x..=right {
        if y < img.height() {
            img.put_pixel(xx, y, color);
        }
        if bottom < img.height() {
            img.put_pixel(xx, bottom, color);
        }
    }

    for yy in y..=bottom {
        if x < img.width() {
            img.put_pixel(x, yy, color);
        }
        if right < img.width() {
            img.put_pixel(right, yy, color);
        }
    }
}

fn draw_text_bitmap(img: &mut RgbaImage, text: &str, x: i32, y: i32, scale: u32, color: Rgba<u8>, max_chars: usize) {
    let scale_i = scale.max(1) as i32;
    let mut cursor_x = x;
    let mut cursor_y = y;
    let mut rendered_chars = 0usize;

    for ch in text.chars() {
        if rendered_chars >= max_chars {
            break;
        }

        if ch == '\n' {
            cursor_x = x;
            cursor_y += (8 * scale_i) + 2;
            continue;
        }

        let glyph = BASIC_FONTS.get(ch).or_else(|| BASIC_FONTS.get('?'));
        if let Some(glyph_rows) = glyph {
            for (row, bits) in glyph_rows.iter().enumerate() {
                for col in 0..8 {
                    // font8x8 rows are bit-packed with LSB=left.
                    let bit_index = col;
                    if (bits >> bit_index) & 1 == 1 {
                        let px_base = cursor_x + col * scale_i;
                        let py_base = cursor_y + (row as i32) * scale_i;
                        for sy in 0..scale_i {
                            for sx in 0..scale_i {
                                let px = px_base + sx;
                                let py = py_base + sy;
                                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                                    img.put_pixel(px as u32, py as u32, color);
                                }
                            }
                        }
                    }
                }
            }
        }

        cursor_x += (8 * scale_i) + 2;
        rendered_chars += 1;

        if cursor_x >= (img.width() as i32 - 10) {
            cursor_x = x;
            cursor_y += (8 * scale_i) + 2;
        }
    }
}

fn fill_background(img: &mut RgbaImage) {
    let h = img.height().max(1);
    for y in 0..img.height() {
        let t = y as f32 / h as f32;
        let r = (16.0 + 18.0 * t) as u8;
        let g = (24.0 + 20.0 * t) as u8;
        let b = (34.0 + 26.0 * t) as u8;
        for x in 0..img.width() {
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
}

fn paint_layout_to_frame(layout: &LayoutOutput, title: &str) -> RgbaImage {
    let mut img = RgbaImage::new(layout.viewport_width.max(320), layout.viewport_height.max(240));
    fill_background(&mut img);
    let img_width = img.width();

    draw_rect(&mut img, 0, 0, img_width, 56, Rgba([8, 14, 22, 255]));
    draw_rect_outline(&mut img, 0, 0, img_width, 56, Rgba([70, 90, 120, 255]));
    draw_text_bitmap(
        &mut img,
        &format!("Catisen Servo Spike: {}", truncate_chars(title, 70)),
        16,
        18,
        1,
        Rgba([175, 225, 255, 255]),
        120,
    );

    for (idx, block) in layout.blocks.iter().enumerate() {
        let bg = match block.tag.as_str() {
            "h1" => Rgba([28, 46, 68, 245]),
            "h2" | "h3" => Rgba([32, 54, 78, 240]),
            "a" => Rgba([24, 44, 88, 235]),
            "code" | "pre" => Rgba([26, 30, 44, 240]),
            _ => Rgba([20, 36, 52, 235]),
        };
        let border = Rgba([82, 122, 160, 255]);

        draw_rect(&mut img, block.x, block.y, block.width, block.height, bg);
        draw_rect_outline(&mut img, block.x, block.y, block.width, block.height, border);

        let label = format!("{} #{}", block.tag, idx + 1);
        draw_text_bitmap(
            &mut img,
            &label,
            block.x as i32 + 8,
            block.y as i32 + 7,
            1,
            Rgba([155, 200, 255, 255]),
            28,
        );

        if block.tag == "img" {
            // Attempt to fetch and draw the actual image instead of just text!
            if let Some(src) = block.attributes.get("src") {
                if src.starts_with("http") {
                    let fetch_result = std::thread::spawn({
                        let url = src.clone();
                        move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                reqwest::get(&url).await.ok()?.bytes().await.ok()
                            })
                        }
                    }).join();
                    
                    if let Ok(Some(bytes)) = fetch_result {
                        if let Ok(fetched_img) = image::load_from_memory(&bytes) {
                            let fetched_img = fetched_img.to_rgba8();
                            let scale_w = block.width.saturating_sub(16).max(10);
                            let fetched_img = image::imageops::resize(&fetched_img, scale_w, block.height.max(100), image::imageops::FilterType::Nearest);
                            let img_w = fetched_img.width();
                            let img_h = fetched_img.height();
                            for y in 0..img_h {
                                for x in 0..img_w {
                                    let px = x + block.x + 8;
                                    let py = y + block.y + 24; // Offset below label
                                    if px < img.width() && py < img.height() {
                                        img.put_pixel(px, py, *fetched_img.get_pixel(x, y));
                                    }
                                }
                            }
                            continue; // Skip drawing the text fallback for images since we drew the real pixels
                        }
                    }
                }
            }
        } else if block.tag == "video" || block.tag == "iframe" {
            // Draw a distinct Media Placeholder box
            let mid_x = block.x + (block.width / 2);
            let mid_y = block.y + (block.height / 2);
            
            // Draw an outer glowing border for media
            draw_rect_outline(&mut img, block.x + 4, block.y + 20, block.width.saturating_sub(8), block.height.saturating_sub(24), Rgba([120, 160, 220, 255]));
            
            // Play Button Triangle or Iframe marker
            let icon_text = if block.tag == "video" { "[ > VIDEO ]" } else { "[ IFRAME ]" };
            draw_text_bitmap(
                &mut img,
                icon_text,
                mid_x as i32 - 40,
                mid_y as i32,
                2,
                Rgba([200, 220, 255, 255]),
                260,
            );
            
            // Subtitle
            draw_text_bitmap(
                &mut img,
                &block.text,
                block.x as i32 + 8,
                block.y as i32 + block.height as i32 - 20,
                1,
                Rgba([180, 200, 220, 255]),
                260,
            );
            continue;
        } else if block.tag == "audio" {
            draw_rect(&mut img, block.x + 8, block.y + 20, block.width.saturating_sub(16), 16, Rgba([40, 60, 80, 255]));
            draw_text_bitmap(
                &mut img,
                "[ = Audio Player = ]",
                block.x as i32 + 16,
                block.y as i32 + 24,
                1,
                Rgba([200, 220, 255, 255]),
                260,
            );
            continue;
        }

        draw_text_bitmap(
            &mut img,
            &block.text,
            block.x as i32 + 8,
            block.y as i32 + 20,
            block.font_scale,
            Rgba([226, 240, 250, 255]),
            260,
        );
    }

    // Viewport Clipping / Scrollbar logic
    if layout.content_height > layout.viewport_height {
        let scrollbar_width = 12u32;
        let sb_x = layout.viewport_width.saturating_sub(scrollbar_width);
        
        // Track
        draw_rect(&mut img, sb_x, 56, scrollbar_width, layout.viewport_height.saturating_sub(56), Rgba([30, 40, 50, 255]));
        
        // Thumb (top aligned since it's a fixed snapshot without real scrolling context yet)
        let ratio = (layout.viewport_height as f32) / (layout.content_height as f32);
        let thumb_height = ((layout.viewport_height.saturating_sub(56)) as f32 * ratio).max(20.0) as u32;
        draw_rect(&mut img, sb_x + 2, 58, scrollbar_width - 4, thumb_height, Rgba([100, 130, 160, 255]));
    }

    img
}

fn browser_candidates() -> Vec<String> {
    let mut candidates = Vec::new();

    if let Ok(custom) = std::env::var("CATISEN_VISUAL_BROWSER") {
        if !custom.trim().is_empty() {
            candidates.push(custom);
        }
    }

    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let user_profile = std::env::var("USERPROFILE").unwrap_or_default();

    candidates.push(format!("{}\\Microsoft\\Edge\\Application\\msedge.exe", program_files));
    candidates.push(format!("{}\\Microsoft\\Edge\\Application\\msedge.exe", program_files_x86));
    candidates.push(format!("{}\\Google\\Chrome\\Application\\chrome.exe", program_files));
    candidates.push(format!("{}\\Google\\Chrome\\Application\\chrome.exe", program_files_x86));
    candidates.push(format!("{}\\Google\\Chrome\\Application\\chrome.exe", local_app_data));
    candidates.push(format!("{}\\BraveSoftware\\Brave-Browser\\Application\\brave.exe", program_files));
    candidates.push(format!("{}\\BraveSoftware\\Brave-Browser\\Application\\brave.exe", local_app_data));
    candidates.push(format!("{}\\Mozilla Firefox\\firefox.exe", program_files));
    candidates.push(format!("{}\\Tor Browser\\Browser\\firefox.exe", program_files));
    candidates.push(format!("{}\\Desktop\\Tor Browser\\Browser\\firefox.exe", user_profile));
    candidates.push(format!("{}\\Tor Browser\\Browser\\firefox.exe", local_app_data));

    candidates.push("msedge".to_string());
    candidates.push("chrome".to_string());
    candidates.push("brave".to_string());
    candidates.push("firefox".to_string());

    candidates
}

fn capture_with_browser(browser_path: &str, url: &str, snapshot_path: &Path) -> bool {
    let snapshot = snapshot_path.to_string_lossy().to_string();
    let browser_name = browser_path.to_ascii_lowercase();

    if browser_name.contains("firefox") {
        if let Ok(mut child) = Command::new(browser_path)
            .args(["--headless", "--screenshot", &snapshot, url])
            .spawn()
        {
            let start = std::time::Instant::now();
            while start.elapsed().as_secs() < 10 {
                if let Ok(Some(s)) = child.try_wait() {
                    return s.success() && snapshot_path.exists();
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            let _ = child.kill();
        }
        return false;
    }

    let user_data_dir = std::env::temp_dir().join(format!("catisen_browser_tmp_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()));
    
    let mut args_new = vec![
        "--headless=new".to_string(),
        "--disable-gpu".to_string(),
        "--no-sandbox".to_string(),
        "--disable-dev-shm-usage".to_string(),
        "--hide-scrollbars".to_string(),
        "--mute-audio".to_string(),
        "--disable-software-rasterizer".to_string(),
        format!("--user-data-dir={}", user_data_dir.to_string_lossy()),
        "--window-size=1366,900".to_string(),
        "--timeout=10000".to_string(),
        "--virtual-time-budget=10000".to_string(),
        format!("--screenshot={}", snapshot),
        url.to_string(),
    ];

    let mut success = false;
    if let Ok(mut child) = Command::new(browser_path).args(&args_new).spawn() {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 12 {
            if let Ok(Some(s)) = child.try_wait() {
                if s.success() && snapshot_path.exists() {
                    success = true;
                }
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let _ = child.kill();
    }

    if !success && !snapshot_path.exists() {
        args_new[0] = "--headless".to_string();
        if let Ok(mut child) = Command::new(browser_path).args(&args_new).spawn() {
            let start = std::time::Instant::now();
            while start.elapsed().as_secs() < 12 {
                if let Ok(Some(s)) = child.try_wait() {
                    if s.success() && snapshot_path.exists() {
                        success = true;
                    }
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            let _ = child.kill();
        }
    }
    
    let _ = std::fs::remove_dir_all(user_data_dir);
    success
}

fn capture_visual_snapshot(url: &str) -> Option<PathBuf> {
    let snapshots_dir = Path::new("target").join("visual_snapshots");
    let _ = std::fs::create_dir_all(&snapshots_dir);

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();

    let snapshot_path = snapshots_dir.join(format!("snapshot-{}.png", ts));

    let mut tried = Vec::new();
    let mut attempt_count = 0;
    
    for browser in browser_candidates() {
        if browser.trim().is_empty() {
            continue;
        }

        let looks_like_path = browser.contains('\\') || browser.contains('/');
        if looks_like_path && !Path::new(&browser).exists() {
            continue;
        }

        tried.push(browser.clone());
        attempt_count += 1;
        
        if capture_with_browser(&browser, url, &snapshot_path) {
            crate::debug_panel::log_msg(&format!("[Visual] Snapshot captured via {}", browser));
            return Some(snapshot_path);
        }
        
        if attempt_count >= 2 {
            break; // Stop after 2 valid browser candidates to avoid hanging the UI
        }
    }

    if !tried.is_empty() {
        crate::debug_panel::log_msg(&format!(
            "[Visual] Snapshot capture failed. Tried: {}",
            tried.join(", ")
        ));
    }

    None
}

fn render_snapshot_bridge(url: &str, html: &str, injected_script: Option<&str>) -> String {
    if let Some(path) = capture_visual_snapshot(url) {
        return format!("{}{}", VISUAL_SNAPSHOT_PREFIX, path.to_string_lossy());
    }

    let script_summary = match injected_script {
        Some(script) => format!("Stealth script injected ({} bytes)", script.len()),
        None => "Stealth script disabled".to_string(),
    };

    let preview = html.chars().take(5000).collect::<String>();
    format!(
        "⚠️ Visual snapshot renderer could not launch a headless browser.\n\
URL: {}\n{}\n\n\
Set CATISEN_VISUAL_BROWSER to a browser executable path (Chrome/Edge/Brave/Firefox/Tor Firefox).\n\
Falling back to preview text below.\n\n\
--- Begin Payload Preview ---\n{}\n--- End Payload Preview ---",
        url,
        script_summary,
        preview
    )
}

// TODO(next PR): Replace spike box painter with real Servo composited rendering pipeline.
// Planned bridge from Servo display list to egui/wgpu:
// 1) Parse and style with Servo components (DOM + style + layout) and collect the final display list.
// 2) Introduce a RenderContext that owns a persistent wgpu device/queue compatible with eframe's backend.
// 3) Translate Servo display items into WebRender primitives, preserving clip/scroll stacks and z-order.
// 4) Execute WebRender frame build into an offscreen texture target (not CPU debug rectangles).
// 5) Export or share the resulting texture with egui as a TextureId without CPU round-tripping.
// 6) Feed input/viewport deltas back into Servo for incremental repaint and partial damage updates.
// 7) Retain spike manifest timings but add compositor timings (display-list build, scene build, GPU submit).
// 8) Gate fallback behavior: if compositor init fails, return existing snapshot bridge marker.
// =========================================================================
// SERVO EMBEDDING (PHASE 2 & 3)
// =========================================================================
use tokio::sync::mpsc;

// A Headless Window that implements Servo's required windowing traits.
// It intercepts WebRender's layout and paint phases, funneling pixels
// into an egui-compatible buffer instead of a physical OS window.
pub enum BrowserEvent {
    Click(f32, f32),
    LoadUrl(String),
}

#[cfg(feature = "servo_real")]
fn init_headless_servo(
    frame_sender: mpsc::UnboundedSender<image::RgbaImage>,
    mut event_receiver: mpsc::UnboundedReceiver<BrowserEvent>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use servo::{
        EventLoopWaker, InputEvent, MouseButtonEvent, MouseButtonAction, MouseButton, WebViewPoint,
        RenderingContext, SoftwareRenderingContext,
        DeviceIntRect, DeviceIntPoint, DeviceIntSize, DevicePoint,
        Opts,
        ServoDelegate, WebViewDelegate,
        ServoBuilder, WebViewBuilder, WebView,
    };
    use winit::dpi::PhysicalSize;
    use std::rc::Rc;

    let preflight = std::panic::catch_unwind(|| {
        SoftwareRenderingContext::new(PhysicalSize::new(width, height))
    });
    match preflight {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            return Err(format!("Software rendering context init failed: {err:?}"));
        }
        Err(_) => {
            return Err("Software rendering context init panicked (EGL/ANGLE unavailable).".to_string());
        }
    }
    
    struct HeadlessWaker;
    impl EventLoopWaker for HeadlessWaker {
        fn clone_box(&self) -> Box<dyn EventLoopWaker> {
            Box::new(HeadlessWaker)
        }
        fn wake(&self) {}
    }

    struct HeadlessDelegate {
        ctx: Rc<SoftwareRenderingContext>,
        frame_sender: mpsc::UnboundedSender<image::RgbaImage>,
    }

    impl ServoDelegate for HeadlessDelegate {}
    impl WebViewDelegate for HeadlessDelegate {
        fn notify_new_frame_ready(&self, _webview: WebView) {
            let size = self.ctx.size();
            let rect = DeviceIntRect::from_origin_and_size(
                DeviceIntPoint::new(0, 0),
                DeviceIntSize::new(size.width as i32, size.height as i32),
            );
            if let Some(img) = self.ctx.read_to_image(rect) {
                let _ = self.frame_sender.send(img);
            }
        }
    }

    crate::debug_panel::log_msg("[ServoSpike] Initialized Software Rendering Context successfully.");

    let default_options = Opts::default();

    crate::debug_panel::log_msg("[ServoSpike] Booting Servo native engine and preparing DOM layout...");

    std::thread::spawn(move || {
        let thread_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let ctx = match SoftwareRenderingContext::new(PhysicalSize::new(width, height)) {
                Ok(ctx) => Rc::new(ctx),
                Err(err) => {
                    crate::debug_panel::log_msg(&format!(
                        "[ServoSpike][ERROR] Software rendering context creation failed in render thread: {err:?}"
                    ));
                    return;
                }
            };

            let delegate = Rc::new(HeadlessDelegate {
                ctx: ctx.clone(),
                frame_sender,
            });

            let servo = ServoBuilder::default()
                .opts(default_options)
                .event_loop_waker(Box::new(HeadlessWaker))
                .build();
            servo.set_delegate(delegate.clone());

            let webview_builder = WebViewBuilder::new(&servo, ctx.clone());
            let webview = webview_builder.delegate(delegate.clone()).build();

            // Polling loop
            loop {
                while let Ok(catisen_event) = event_receiver.try_recv() {
                    match catisen_event {
                        BrowserEvent::LoadUrl(url_str) => {
                            if let Ok(url) = url::Url::parse(&url_str) {
                                webview.load(url);
                            }
                        }
                        BrowserEvent::Click(x, y) => {
                            let pt = WebViewPoint::Device(DevicePoint::new(x, y));
                            webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                                MouseButtonAction::Down,
                                MouseButton::Left,
                                pt
                            )));
                            webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                                MouseButtonAction::Up,
                                MouseButton::Left,
                                pt
                            )));
                        }
                    }
                }

                servo.spin_event_loop();
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }));

        if thread_result.is_err() {
            crate::debug_panel::log_msg(
                "[ServoSpike][ERROR] Servo render thread panicked. Falling back to snapshot/static output."
            );
        }
    });

    Ok(())
}

#[cfg(not(feature = "servo_real"))]
fn init_headless_servo(
    _frame_sender: mpsc::UnboundedSender<image::RgbaImage>,
    _event_receiver: mpsc::UnboundedReceiver<BrowserEvent>,
    _width: u32,
    _height: u32,
) -> Result<(), String> {
    Ok(())
}

fn render_servo_spike(url: &str, html: &str, injected_script: Option<&str>) -> (
    String,
    Option<image::RgbaImage>,
    Option<mpsc::UnboundedReceiver<image::RgbaImage>>,
    Option<mpsc::UnboundedSender<BrowserEvent>>,
) {
    crate::debug_panel::log_msg("[ServoSpike] Starting DOM/style/layout/paint pipeline.");

    let viewport_width = std::env::var("CATISEN_VISUAL_WIDTH")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1366)
        .clamp(640, 2560);

    let viewport_height = std::env::var("CATISEN_VISUAL_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(900)
        .clamp(420, 1440);

    // Create the channels for Catisen UI to communicate with Servo thread natively
    let (tx, rx) = mpsc::unbounded_channel::<image::RgbaImage>();
    let (evt_tx, event_rx) = mpsc::unbounded_channel::<BrowserEvent>();

    let mut live_servo_available = true;
    if let Err(e) = init_headless_servo(tx, event_rx, viewport_width, viewport_height) {
        crate::debug_panel::log_msg(&format!("[ServoSpike][ERROR] Servo Init Failed: {}", e));
        live_servo_available = false;
    }

    if !live_servo_available {
        crate::debug_panel::log_msg("[ServoSpike] Falling back to snapshot bridge because live Servo initialization is unavailable.");
        let fallback = render_snapshot_bridge(url, html, injected_script);
        if fallback.starts_with(VISUAL_SNAPSHOT_PREFIX) {
            return (fallback, None, None, None);
        }
    }


    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let spike_root = Path::new("target").join("servo_spike");
    let manifests_dir = spike_root.join("manifests");
    let frames_dir = spike_root.join("frames");
    let _ = std::fs::create_dir_all(&manifests_dir);
    let _ = std::fs::create_dir_all(&frames_dir);

    let viewport_width = std::env::var("CATISEN_VISUAL_WIDTH")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1366)
        .clamp(640, 2560);

    let viewport_height = std::env::var("CATISEN_VISUAL_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(900)
        .clamp(420, 1440);

    let total_start = Instant::now();

    let dom_start = Instant::now();
    let document = Html::parse_document(html);
    let dom_parse = parse_dom_stage(&document);
    let dom_parse_ms = dom_start.elapsed().as_millis();

    let style_start = Instant::now();
    let style_resolve = parse_style_stage(&document);
    let style_resolve_ms = style_start.elapsed().as_millis();

    let layout_start = Instant::now();
    let layout = layout_stage(&dom_parse, viewport_width, viewport_height);
    let layout_ms = layout_start.elapsed().as_millis();

    let frame_path = frames_dir.join(format!("frame-{}.png", timestamp_ms));
    let paint_start = Instant::now();
    let img = paint_layout_to_frame(&layout, &dom_parse.title);
    let paint_ms = paint_start.elapsed().as_millis();
    if let Err(err) = img.save(&frame_path) {
        crate::debug_panel::log_msg(&format!(
            "[ServoSpike][WARN] Failed to persist frame image {}: {}",
            frame_path.to_string_lossy(),
            err
        ));
    }

    let total_ms = total_start.elapsed().as_millis();

    let timings = StageTimings {
        dom_parse_ms,
        style_resolve_ms,
        layout_ms,
        paint_ms,
        total_ms,
    };

    let manifest = SpikeManifest {
        timestamp_ms,
        url: url.to_string(),
        html_bytes: html.len(),
        injected_script_bytes: injected_script.map(|s| s.len()).unwrap_or(0),
        render_backend: "servo-spike".to_string(),
        stage_timings: timings,
        dom_parse: dom_parse.clone(),
        style_resolve,
        layout,
        output_frame_png: frame_path.to_string_lossy().to_string(),
        note: "Composited frame generated from staged DOM/style/layout pipeline on spike path.".to_string(),
    };

    let manifest_path = manifests_dir.join(format!("frame-{}.json", timestamp_ms));
    if let Ok(serialized) = serde_json::to_string_pretty(&manifest) {
        if std::fs::write(&manifest_path, serialized).is_ok() {
            crate::debug_panel::log_msg(&format!(
                "[ServoSpike] Manifest written: {}",
                manifest_path.to_string_lossy()
            ));
        }
    }

    crate::debug_panel::log_msg(&format!(
        "[ServoSpike] Stage timings ms: dom={} style={} layout={} paint={} total={}",
        dom_parse_ms, style_resolve_ms, layout_ms, paint_ms, total_ms
    ));

    // Also send an initial LoadUrl event to kick off parsing in real Servo (if servo is linked and enabled)
    let _ = evt_tx.send(BrowserEvent::LoadUrl(url.to_string()));

    (format!("{}{}", VISUAL_SNAPSHOT_PREFIX, frame_path.to_string_lossy()), Some(img), Some(rx), Some(evt_tx))
}

pub fn render_visual_page(url: &str, html: &str, injected_script: Option<&str>) -> (
    String,
    Option<image::RgbaImage>,
    Option<mpsc::UnboundedReceiver<image::RgbaImage>>,
    Option<mpsc::UnboundedSender<BrowserEvent>>,
) {
    match selected_visual_engine() {
        VisualEngine::SnapshotBridge => (render_snapshot_bridge(url, html, injected_script), None, None, None),
        VisualEngine::ServoSpike => render_servo_spike(url, html, injected_script),
    }
}
