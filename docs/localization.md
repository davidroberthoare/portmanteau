# Localization (interface language)

Portmanteau's menus, buttons, headings and messages can be shown in other languages. Languages are plain JSON files, so anyone can add or correct one without touching the program. What is translated is the **interface**. A writer's own text, project data and every export are never translated.

## For writers: using a language

**Settings > Preferences > Interface language.** Pick one from the list. **Import…** adds a language file someone sent you. **Export template…** saves an empty file you can translate. Anything not yet translated stays in English, so a half-finished language is perfectly usable.

Language files live in a `languages` folder beside the app's settings (`~/.config/portmanteau/languages` on Linux; the exact location is shown on the Settings page).

## For translators: making or fixing a language

1. In Settings, press **Export template…**. (Or copy `assets/languages/template.json` from the source.)
2. Edit it in any text editor:
   ```json
   {
     "code": "fr",
     "name": "Français",
     "strings": {
       "File": "Fichier",
       "{count} words in the project": "{count} mots dans le projet"
     }
   }
   ```
   - `code`: letters, digits, `-` or `_` (for example `fr`, `pt-BR`). It is also the file name (`fr.json`).
   - `name`: the language written in itself. It is what the picker shows.
   - `strings`: each **English text** (the key, exactly as it appears in the template) and its translation. Leave a translation as `""` (or delete the line) to keep the English.
   - `{placeholders}` such as `{count}` or `{title}` must be kept, exactly, in the translation. They are replaced by the program. You can move them wherever the grammar needs.
3. Put the file in the `languages` folder (or use **Import…**) and choose it in Settings.

A file whose `code` matches a built-in language (for example `fr`) **replaces** that language, which is how you correct a built-in translation without rebuilding the app. English is built in and cannot be replaced. Files that cannot be read are skipped, and the Settings page says why.

To contribute a language to the program itself, add `assets/languages/<code>.json` and list it in `BUILTIN` in `src/i18n.rs`.

### Limits to know about

- **Fonts.** The app bundles Latin fonts (plus a symbol font). Languages written in other scripts (Chinese, Japanese, Korean, Arabic, Hebrew, Hindi…) need glyphs the app does not ship yet, so they would show empty boxes. Right-to-left layout is not supported.
- **Not translated:** your text and notes, names you type (labels, tags, characters), file formats, dates, and the Command desk's technical commands. Document status names and labels (Draft, Revised, Opening…) are translated for display only; the saved value stays English so projects stay compatible between languages.
- **Plurals.** Texts carry a `{count}` and are not pluralised; word them so they read well for any number (the French file uses forms like "ajouté(s)").

## For developers: adding text to the program

Write every user-visible English text through `tr`:

```rust
use crate::i18n::{tr, trf};

ui.button(tr("Save checkpoint"));
RichText::new(trf("{count} words in the project", &[("count", n.to_string())]));
```

- `tr("literal")` returns the active language's text for that English text (or the English). It returns `&'static str`, so it can be passed anywhere a `&str` is.
- `trf("template {name}", &[("name", value)])` translates a text with placeholders and fills them in. Never build a sentence with `format!` and translate the pieces; translate the whole sentence with placeholders.
- `tr_dyn(&string)` is for text that is not a literal (a value from data). Prefer listing the literals with `tr(..)` next to it so the template picks them up; see the status dropdown in `src/ui/inspector.rs`.
- The English text is the key. **Changing an English text changes its key**, so translations of the old wording stop applying until the language files are updated. Rewording is therefore a localisation decision, not just a copy edit.
- After adding or changing texts run
  ```sh
  UPDATE_LANGUAGE_TEMPLATE=1 cargo test the_template
  cargo test
  ```
  (twice: the first run rewrites `assets/languages/template.json`, which is compiled in). The tests fail when the template is out of date, and when a built-in translation has a key that no longer exists or has lost a `{placeholder}`.
- Run the app in a language for a screenshot with `PORTMANTEAU_LANG=fr` (together with `--screenshot`). Screenshot runs never write the user's settings.

## How it works

`src/i18n.rs` holds the active language as a map from English text to translation. `Language` is read from JSON (`from_json` validates the code and name and ignores empty translations). `src/i18n/files.rs` finds language files in the `languages` folder, mirroring the themes folder. The choice is stored in `AppSettings.interface_language`. Translations are interned once for the life of the program so `tr` can hand out `&'static str` without allocating on every frame.

## Status

First pass: menus, rail, page headings, Settings, Compile, the inspector, the binder and most buttons and messages are wrapped. The built-in French translation is a first draft written by an AI assistant and needs a native speaker's review. Some dynamic messages (for example error text built from system errors) are still English-only.
