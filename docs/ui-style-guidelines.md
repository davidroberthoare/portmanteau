# UI Style Direction — Light, Quiet, Mac-Native

**Status:** Saved for a future UI redesign. This is guidance, not an instruction
to change the current interface during the rich-text-model work.

## Goal

Make Portmanteau feel like a calm writing and book-making tool, not a
high-tech dashboard. The manuscript is the visual focus. Controls should be
quiet, sparse and discoverable when needed.

Use Vellum as the primary reference for the writing/editor/preview relationship,
iA Writer for focus and restraint, Ulysses for document-library navigation, and
Apple's Human Interface Guidelines for native sidebar and toolbar behavior.
The supplied second and third references are useful for whitespace and clear
surface grouping, but their saturated blue sidebar and colorful metric cards
should not be copied literally.

## Core principles

1. **Writing comes first.** Give the manuscript the largest, quietest area.
   Chrome should recede while editing.
2. **Use hierarchy, not decoration.** Prefer alignment, spacing, typography and
   grouping to thick borders, shadows, gradients, badges or piles of buttons.
3. **Show controls when relevant.** Keep common actions in one compact,
   context-sensitive toolbar. Move infrequent actions into menus, inspectors or
   contextual controls.
4. **Use native-feeling behavior.** Respect familiar Mac menu, keyboard,
   selection, focus and panel conventions without imitating platform branding.
5. **Offer focus without trapping the writer.** The binder, inspector and
   preview can be hidden or restored; focus mode should reduce clutter, not
   change document behavior.
6. **Keep the interface accessible.** Maintain readable contrast, visible
   keyboard focus, predictable hit targets and text labels for unfamiliar icons.

## Window composition

- **Top toolbar:** one light, low-height row for document context and a few
  relevant actions. Group related controls; avoid a full-width dark bar.
- **Binder/sidebar:** optional, collapsible, neutral surface. Use concise labels,
  restrained disclosure controls and a subtle selection tint. Keep the visible
  hierarchy shallow; avoid turning the rail into a wall of bright icons.
- **Editor:** central and dominant. Use a warm-white or paper-like surface with
  generous margins. Avoid a heavy card outline or dramatic drop shadow around
  every page.
- **Inspector/preview:** secondary and dismissible. Show one when useful rather
  than permanently crowding the writing surface. A live preview should be easy
  to open and should track the current selection or paragraph.
- **Status:** keep word count, save state and goals in a quiet footer or compact
  status area, not in large dashboard cards.
- **Panels:** three-column binder/editor/preview layouts are acceptable on wide
  windows, but narrow layouts should collapse secondary panels rather than
  squeeze the editor.

## Color and surfaces

Use theme tokens rather than hard-coded colors. A starting direction:

| Token | Suggested direction | Use |
| --- | --- | --- |
| Window background | warm light gray, around `#F4F5F6` | App chrome |
| Editor paper | soft white / warm white, around `#FBFAF7` | Manuscript |
| Primary text | charcoal, around `#282B30` | Body and controls |
| Secondary text | muted gray, around `#737981` | Metadata and hints |
| Divider/border | very light gray, around `#E4E6E8` | Separation only |
| Primary accent | muted blue-green, around `#5B8F8A` | Selection, primary action |
| Warm accent | subdued brass, around `#C99A52` | Occasional writing/status accent |
| Error | restrained red | Errors and spelling only |

- Default to a **light appearance**. A dark theme can remain optional, but should
  not define the app's identity.
- Use color sparingly: active navigation, selected text/style, primary actions,
  and warnings. Avoid saturating a whole sidebar or filling every card with a
  different color.
- Keep selected rows legible with a pale tint or a fine accent rule, not a
  glowing outline.
- Avoid gradients, glass effects and decorative shadows as structural cues.

## Typography and spacing

- Use the manuscript's chosen typeface in the editor: serif for prose and
  screenplay Courier/monospace where the style sheet calls for it.
- Use a restrained sans-serif for chrome. Reserve monospace outside the
  manuscript for code-like or numeric data.
- Establish a compact spacing scale (for example 4, 8, 12, 16 and 24 points).
  Use larger gaps between areas and smaller gaps within groups.
- Keep toolbar controls visually light and consistent. Icons should be
  monochrome by default; color is a state cue, not ornament.
- Preserve clear focus and hover states without using large filled buttons for
  every action.

## Interaction and progressive disclosure

- Put high-frequency formatting tools close to the writing surface; keep them
  in a single compact row.
- Use menus for infrequent file, project and export operations.
- Keep advanced style-sheet and layout controls in a dedicated inspector or
  settings panel, not permanently visible beside the text.
- Use tooltips for secondary explanations, but keep important controls named.
- Make preview, binder and inspector panels independently show/hide where
  practical. Remember the writer's last layout.
- Maintain keyboard-first workflows: selection, navigation, style shortcuts,
  next-style behavior and Escape must work without mouse-only controls.

## Product-specific direction

- Preserve the established editorial identity: paper-like writing surface,
  serif prose, monospace screenplay, restrained accent color.
- Reduce the current always-visible button count. Favor one relevant toolbar
  that changes with the selected paragraph/style over persistent rows of
  unrelated controls.
- Keep the screenplay palette compact; show the selected style and make the
  rest available through a menu or keyboard shortcut if the toolbar becomes
  crowded.
- Treat the binder as navigation, not a colorful dashboard. Use light group
  labels and quiet counts.
- Make the preview feel like a clean book page. Avoid device frames or thick
  black outlines except when the user explicitly selects a device preview.
- Use warm paper and a restrained accent inspired by the supplied references;
  do not adopt the yellow canvas or saturated blue navigation as global fills.

## Review checklist for the future redesign

- Can a writer identify the current document and begin typing without scanning
  a dense toolbar?
- Does the editor remain the focal point at 1440×930 and at smaller window
  sizes?
- Can binder, inspector and preview be hidden without losing access to their
  actions?
- Are selection, focus, hover and disabled states visible but quiet?
- Are rich-text, screenplay-style and preview controls placed near the content
  they affect?
- Are color and contrast accessible in both light and optional dark themes?
- Does the app still feel distinctive and editorial rather than like a generic
  dashboard?

## References

- [Vellum Text Editor](https://help.vellum.pub/text-editor/) — central editor,
  compact formatting controls and synchronized preview.
- [Vellum](https://vellum.pub/) — book-first editing and preview workflow.
- [Ulysses](https://ulysses.app/) — restrained writing surface and document
  library organization.
- [iA Writer](https://ia.net/writer) — focus mode and intentional reduction of
  on-screen controls.
- [Apple Human Interface Guidelines: Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars)
  — shallow navigation hierarchy and sidebar usage.
- [Apple Human Interface Guidelines: Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)
  — grouping and purpose of toolbar controls.
- [Apple Human Interface Guidelines: Layout](https://developer.apple.com/design/human-interface-guidelines/layout)
  — visual hierarchy through deliberate alignment and spacing.
