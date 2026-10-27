//! The viewer's type system: Inter for reading, JetBrains Mono for small
//! uppercase labels, and a short fade-and-rise whenever overlay text appears.
//!
//! Inter replaces Bevy's built-in FiraMono as the default font, so any text
//! spawned with `TextFont::default()` reads in Inter without opting in.

use bevy::{
    asset::uuid_handle,
    prelude::*,
    text::FontWeight,
    ui::UiSystems,
};

const REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");

const MEDIUM_FONT: Handle<Font> = uuid_handle!("6f1d2b0e-6b8f-4a43-9d38-0c3f1a7b5e21");
const SEMIBOLD_FONT: Handle<Font> = uuid_handle!("a3c4e8d2-1f57-4b9e-8a60-2d7e9b4c3f12");
const MONO_FONT: Handle<Font> = uuid_handle!("d9e2f1a0-3c6b-4e8d-b7a5-5f4c2e1d0b93");

/// Seconds a reveal takes once its delay has passed.
const REVEAL_SECONDS: f32 = 0.42;

/// Near-white for headings and names.
pub(crate) const INK: Color = Color::srgb(0.93, 0.95, 0.97);
/// Body copy: bright enough to read, quieter than headings.
pub(crate) const BODY: Color = Color::srgb(0.76, 0.81, 0.86);
/// Eyebrows, hints and metadata.
pub(crate) const MUTED: Color = Color::srgb(0.53, 0.60, 0.67);

pub(crate) struct TypographyPlugin;

impl Plugin for TypographyPlugin {
    fn build(&self, app: &mut App) {
        let mut fonts = app.world_mut().resource_mut::<Assets<Font>>();
        for (id, bytes) in [
            (AssetId::default(), REGULAR),
            (MEDIUM_FONT.id(), MEDIUM),
            (SEMIBOLD_FONT.id(), SEMIBOLD),
            (MONO_FONT.id(), MONO),
        ] {
            let font = Font::try_from_bytes(bytes.to_vec()).expect("bundled font parses");
            fonts.insert(id, font).expect("uuid font ids always insert");
        }
        app.add_systems(PostUpdate, reveal.before(UiSystems::Layout));
    }
}

/// Reading text: Inter Regular.
pub(crate) fn body(size: f32) -> TextFont {
    TextFont::from_font_size(size)
}

/// Names and emphasis: Inter Medium.
pub(crate) fn medium(size: f32) -> TextFont {
    face(MEDIUM_FONT, FontWeight::MEDIUM, size)
}

/// Headings: Inter SemiBold.
pub(crate) fn heading(size: f32) -> TextFont {
    face(SEMIBOLD_FONT, FontWeight::SEMIBOLD, size)
}

/// Small uppercase labels: JetBrains Mono Medium, whose wide set reads as
/// tracked caps.
pub(crate) fn mono(size: f32) -> TextFont {
    face(MONO_FONT, FontWeight::MEDIUM, size)
}

/// Splits light Markdown emphasis (`**bold**`) into plain and strong runs, so
/// agent-written text never shows its asterisks.
pub(crate) fn emphasis_runs(text: &str) -> Vec<(String, bool)> {
    let mut runs = Vec::new();
    for (i, part) in text.split("**").enumerate() {
        if !part.is_empty() {
            runs.push((part.to_owned(), i % 2 == 1));
        }
    }
    runs
}

/// A text block whose `**bold**` runs set in `strong` and the rest in `plain`.
pub(crate) fn rich(text: &str, plain: TextFont, strong: TextFont, color: Color) -> impl Bundle {
    let spans: Vec<_> = emphasis_runs(text)
        .into_iter()
        .map(|(run, bold)| {
            let font = if bold { strong.clone() } else { plain.clone() };
            (TextSpan::new(run), font, TextColor(color))
        })
        .collect();
    (
        Text::new(""),
        plain,
        TextColor(color),
        Children::spawn(SpawnIter(spans.into_iter())),
    )
}

fn face(font: Handle<Font>, weight: FontWeight, size: f32) -> TextFont {
    TextFont {
        font,
        weight,
        ..TextFont::from_font_size(size)
    }
}

/// Fades an overlay and its descendants in from a few pixels below once
/// `delay` seconds have passed. A descendant with its own `Reveal` keeps its
/// own timing. The clock waits while a label is held for a transition.
#[derive(Component, Clone, Debug)]
pub(crate) struct Reveal {
    delay: f32,
    rise: f32,
    age: f32,
    seconds: f32,
    base: Option<Vec<Base>>,
}

impl Reveal {
    pub fn after(delay: f32) -> Self {
        Self {
            delay,
            rise: 10.0,
            age: 0.0,
            seconds: REVEAL_SECONDS,
            base: None,
        }
    }

    pub fn rise(mut self, px: f32) -> Self {
        self.rise = px;
        self
    }

    pub fn seconds(mut self, seconds: f32) -> Self {
        self.seconds = seconds;
        self
    }
}

#[derive(Clone, Debug)]
struct Base {
    entity: Entity,
    text: Option<f32>,
    background: Option<f32>,
    border: Option<[f32; 4]>,
}

#[allow(clippy::type_complexity)]
fn reveal(
    mut commands: Commands,
    time: Res<Time>,
    mut reveals: Query<
        (Entity, &mut Reveal, Option<&mut UiTransform>),
        Without<crate::transition::Held>,
    >,
    own: Query<(), With<Reveal>>,
    children: Query<&Children>,
    mut colors: Query<(
        Option<&mut TextColor>,
        Option<&mut BackgroundColor>,
        Option<&mut BorderColor>,
    )>,
) {
    for (entity, mut reveal, transform) in &mut reveals {
        if reveal.base.is_none() {
            let mut base = Vec::new();
            collect(entity, entity, &own, &children, &colors, &mut base);
            reveal.base = Some(base);
        }
        reveal.age += time.delta_secs();
        let t = ((reveal.age - reveal.delay) / reveal.seconds).clamp(0.0, 1.0);
        let fade = ease_out_cubic(t);
        let lift = 1.0 - ease_out_quint(t);
        if let Some(mut transform) = transform {
            transform.translation.y = px(reveal.rise * lift);
        }
        for base in reveal.base.iter().flatten() {
            let Ok((text, background, border)) = colors.get_mut(base.entity) else {
                continue;
            };
            if let (Some(mut color), Some(alpha)) = (text, base.text) {
                color.0.set_alpha(alpha * fade);
            }
            if let (Some(mut color), Some(alpha)) = (background, base.background) {
                color.0.set_alpha(alpha * fade);
            }
            if let (Some(mut color), Some([top, right, bottom, left])) = (border, base.border) {
                color.top.set_alpha(top * fade);
                color.right.set_alpha(right * fade);
                color.bottom.set_alpha(bottom * fade);
                color.left.set_alpha(left * fade);
            }
        }
        if t >= 1.0 {
            commands.entity(entity).try_remove::<Reveal>();
        }
    }
}

#[allow(clippy::type_complexity)]
fn collect(
    root: Entity,
    entity: Entity,
    own: &Query<(), With<Reveal>>,
    children: &Query<&Children>,
    colors: &Query<(
        Option<&mut TextColor>,
        Option<&mut BackgroundColor>,
        Option<&mut BorderColor>,
    )>,
    out: &mut Vec<Base>,
) {
    if entity != root && own.contains(entity) {
        return;
    }
    if let Ok((text, background, border)) = colors.get(entity) {
        out.push(Base {
            entity,
            text: text.map(|c| c.0.alpha()),
            background: background.map(|c| c.0.alpha()),
            border: border.map(|c| {
                [
                    c.top.alpha(),
                    c.right.alpha(),
                    c.bottom.alpha(),
                    c.left.alpha(),
                ]
            }),
        });
    }
    for &child in children.get(entity).into_iter().flatten() {
        collect(root, child, own, children, colors, out);
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emphasis_drops_the_asterisks() {
        assert_eq!(
            emphasis_runs("**Line items** become revenue"),
            vec![
                ("Line items".to_owned(), true),
                (" become revenue".to_owned(), false)
            ]
        );
    }
}
