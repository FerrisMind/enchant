//! Magic UI widget ports for `iced`.
//!
//! # API canon (pre-1.0)
//!
//! New and refurbished widgets follow the builder-first style of
//! [`BorderBeam`] and [`ShimmerButton`] (inspired by `iced-shadcn-v2`):
//!
//! - private fields, fluent `#[must_use]` setters, validation in setters
//! - `#[non_exhaustive]` on public enums
//! - terminal `into_element` / `view` (effects take `elapsed: Duration`)
//! - Magic UI capability parity without leaking CSS/`className`
//!
//! Prefer these two modules as the template when migrating remaining `*Props`
//! free-function widgets.

pub mod animated_beam;
pub mod animated_circular_progress_bar;
pub mod animated_gradient_text;
pub mod animated_grid_pattern;
pub mod animated_list;
pub mod animated_shiny_text;
pub mod animated_subscribe_button;
pub mod animated_theme_toggler;
pub mod aurora_text;
pub mod avatar_circles;
pub mod backlight;
pub mod blur_fade;
pub mod border_beam;
pub mod border_track;
pub mod code_comparison;
pub mod comic_text;
pub mod confetti;
pub mod cool_mode;
pub mod dia_text_reveal;
pub mod dock;
pub mod dot_pattern;
pub mod dotted_map;
pub mod file_tree;
pub mod flickering_grid;
pub mod glare_hover;
pub mod globe;
pub mod glyph_matrix;
pub mod grid_pattern;
pub mod hero_video_dialog;
pub mod hexagon_pattern;
pub mod highlighter;
pub mod hyper_text;
pub mod icon_cloud;
pub mod interactive_grid_pattern;
pub mod interactive_hover_button;
pub mod kinetic_text;
pub mod lens;
pub mod light_rays;
pub mod line_shadow_text;
pub mod magic_card;
pub mod marquee;
pub mod meteors;
pub mod mockups;
pub mod morphing_text;
pub mod motion_grid;
pub mod neon_gradient_card;
pub mod noise_texture;
pub mod number_ticker;
pub mod orbiting_circles;
pub mod particles;
pub mod pixel_image;
pub mod pointer;
pub mod progressive_blur;
pub mod pulsating_button;
pub mod rainbow_button;
pub mod retro_grid;
pub mod ripple;
pub mod ripple_button;
pub mod scroll_based_velocity;
pub mod scroll_progress;
pub mod shimmer_button;
pub mod shine_border;
pub mod shiny_button;
pub mod smooth_cursor;
pub mod sparkles_text;
pub mod spinning_text;
pub mod striped_pattern;
pub mod terminal;
pub mod text_3d_flip;
pub mod text_animate;
pub mod text_element;
pub mod text_reveal;
pub mod tweet_card;
pub mod typing_animation;
pub mod video_text;
pub mod warp_background;
pub mod word_rotate;

pub use animated_beam::{AnimatedBeamProps, animated_beam};
pub use animated_circular_progress_bar::{
    AnimatedCircularProgressBarProps, AnimatedCircularProgressBarState,
    animated_circular_progress_bar,
};
pub use animated_gradient_text::{AnimatedGradientTextProps, animated_gradient_text};
pub use animated_grid_pattern::{AnimatedGridPatternProps, animated_grid_pattern};
pub use animated_list::{AnimatedListProps, AnimatedListState, animated_list};
pub use animated_shiny_text::{AnimatedShinyTextProps, animated_shiny_text};
pub use animated_subscribe_button::{
    AnimatedSubscribeButtonProps, AnimatedSubscribeButtonState, animated_subscribe_button,
};
pub use animated_theme_toggler::{
    AnimatedThemeTogglerController, AnimatedThemeTogglerProps, animated_theme_toggler,
    animated_theme_toggler_with_controller, theme_transition_overlay,
};
pub use aurora_text::{AuroraTextProps, aurora_text};
pub use avatar_circles::{AvatarCircleEntry, AvatarCirclesProps, avatar_circles};
pub use backlight::{BacklightProps, BacklightSvgGlow, backlight};
pub use blur_fade::{BlurFadeDirection, BlurFadeProps, BlurFadeState, blur_fade};
pub use border_beam::{BorderBeam, BorderBeamEase};
pub use code_comparison::{CodeComparisonProps, code_comparison};
pub use comic_text::{ComicTextProps, ComicTextState, comic_text};
pub use confetti::{Confetti, ConfettiParticleShape, ConfettiState};
pub use cool_mode::{CoolMode, CoolParticleKind};
pub use dia_text_reveal::{
    DEFAULT_DIA_COLORS, DiaTextRevealProps, DiaTextRevealState, dia_text_reveal,
    dia_text_reveal_companion,
};
pub use dock::{
    Dock, DockAction, DockDirection, DockEntry, DockIcon, DockState, DockValues,
    dock_entry_icon_count, dock_icon, dock_separator,
};
pub use dot_pattern::{DotPatternProps, dot_pattern};
pub use dotted_map::{
    DottedMapMarker, DottedMapOverlay, DottedMapProps, DottedMapState, dotted_map,
};
pub use file_tree::{
    FileTreeAction, FileTreeElement, FileTreeKind, FileTreeProps, FileTreeSort, FileTreeState,
    file_tree,
};
pub use flickering_grid::{FlickeringGridProps, flickering_grid};
pub use glare_hover::{GlareHoverAction, GlareHoverProps, GlareHoverState, glare_hover};
pub use globe::{
    GlobeAction, GlobeDemoColors, GlobeMarker, GlobeProps, GlobeState, blend_globe_demo_colors,
    globe, globe_demo, globe_demo_colors_blended, globe_demo_colors_for_controller,
    globe_demo_colors_for_mode, globe_demo_colors_from_theme,
};
pub use glyph_matrix::{GlyphMatrixProps, glyph_matrix};
pub use grid_pattern::{GridPatternProps, grid_pattern};
pub use hero_video_dialog::{
    HeroVideoAnimationStyle, HeroVideoDialogProps, HeroVideoDialogState, hero_video_dialog,
};
pub use hexagon_pattern::{HexagonDirection, HexagonPatternProps, hexagon_pattern};
pub use highlighter::{HighlighterAction, HighlighterProps, HighlighterState, highlighter};
pub use hyper_text::{HyperTextAction, HyperTextProps, HyperTextState, hyper_text};
pub use icon_cloud::{IconCloudAction, IconCloudItem, IconCloudProps, IconCloudState, icon_cloud};
pub use interactive_grid_pattern::{
    InteractiveGridPatternAction, InteractiveGridPatternProps, InteractiveGridPatternState,
    interactive_grid_pattern,
};
pub use interactive_hover_button::{
    InteractiveHoverButton, InteractiveHoverEase, InteractiveHoverIcon,
};
pub use kinetic_text::{KineticTextProps, KineticTextRole, KineticTextState, kinetic_text};
pub use lens::{Lens, LensAction, LensState, LensValues, LensVisibility};
pub use light_rays::{LightRaysProps, light_rays};
pub use line_shadow_text::{LineShadowTextProps, line_shadow_color_for_mode, line_shadow_text};
pub use magic_card::{MagicCardMode, MagicCardProps, magic_card};
pub use marquee::{MarqueeItem, MarqueeOrientation, MarqueeProps, marquee, marquee_indexed};
pub use meteors::{MeteorsProps, meteors};
pub use mockups::{AndroidProps, IphoneProps, SafariMode, SafariProps, android, iphone, safari};
pub use morphing_text::{MorphingTextProps, morphing_text};
pub use motion_grid::{FrameDot, MotionGrid, MotionGridEase};
pub use neon_gradient_card::{NeonColors, NeonGradientCardProps, neon_gradient_card};
pub use noise_texture::{NoiseTextureProps, noise_texture, noise_texture_layer};
pub use number_ticker::{NumberTicker, NumberTickerDirection};
pub use orbiting_circles::{OrbitingCirclesProps, orbiting_circles};
pub use particles::{ParticlesAction, ParticlesProps, ParticlesState, particles};
pub use pixel_image::{PixelGrid, PixelGridPreset, PixelImageProps, PixelImageState, pixel_image};
pub use pointer::{PointerAction, PointerProps, PointerState, pointer};
pub use progressive_blur::{ProgressiveBlurPosition, ProgressiveBlurProps, progressive_blur};
pub use pulsating_button::{PulsatingButtonProps, PulsatingButtonVariant, pulsating_button};
pub use rainbow_button::{
    RainbowButtonProps, RainbowButtonSize, RainbowButtonVariant, rainbow_button,
};
pub use retro_grid::{RetroGridProps, retro_grid};
pub use ripple::{RippleProps, ripple};
pub use ripple_button::{RippleButtonAction, RippleButtonProps, RippleButtonState, ripple_button};
pub use scroll_based_velocity::{
    ScrollVelocityContainerProps, ScrollVelocityDirection, ScrollVelocityItem,
    ScrollVelocityRowAction, ScrollVelocityRowProps, ScrollVelocityRowState,
    scroll_velocity_container, scroll_velocity_row,
};
pub use scroll_progress::{ScrollProgressProps, scroll_progress};
pub use shimmer_button::ShimmerButton;
pub use shine_border::{ShineBorderProps, shine_border};
pub use shiny_button::{ShinyButtonProps, shiny_button, shiny_text_color_for_mode};
pub use smooth_cursor::{
    SmoothCursorAction, SmoothCursorProps, SmoothCursorSpringConfig, SmoothCursorState,
    smooth_cursor,
};
pub use sparkles_text::{SparklesColors, SparklesTextProps, sparkles_text};
pub use spinning_text::{
    SpinningTextProps, SpinningTextTransition, SpinningTextVariantState, SpinningTextVariants,
    spinning_text,
};
pub use striped_pattern::{StripedPatternDirection, StripedPatternProps, striped_pattern};
pub use terminal::{
    TerminalLine, TerminalLineMode, TerminalProps, TerminalSegment, TerminalState, terminal,
};
pub use text_3d_flip::{
    Text3DFlipAction, Text3DFlipDirection, Text3DFlipProps, Text3DFlipStaggerFrom, Text3DFlipState,
    text_3d_flip,
};
pub use text_animate::{
    TextAnimateBy, TextAnimatePreset, TextAnimateProps, TextAnimateState, TextAnimateVariantState,
    TextAnimateVariants, text_animate,
};
pub use text_element::TextElementRole;
pub use text_reveal::{TextRevealProps, text_reveal};
pub use tweet_card::{
    ClientTweetCardAction, ClientTweetCardProps, ClientTweetCardState, ClientTweetCardStatus,
    TweetAsset, TweetAuthor, TweetCardData, TweetCardProps, TweetMedia, client_tweet_card,
    tweet_card,
};
pub use typing_animation::{
    TypingAnimationCursorStyle, TypingAnimationProps, TypingAnimationState, typing_animation,
};
pub use video_text::{VideoTextProps, VideoTextSource, VideoTextState, video_text};
pub use warp_background::{WarpBackgroundProps, warp_background};
pub use word_rotate::{WordRotateProps, WordRotateState, word_rotate};
