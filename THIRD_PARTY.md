# Third-party material

Portmanteau's own code is licensed under the GNU GPL v3 (see `LICENSE`).
The files below are bundled in `assets/` and keep their original licences.

| Files | What | Licence |
| --- | --- | --- |
| `assets/LiberationSans-*.ttf`, `LiberationSerif-*.ttf`, `LiberationMono-Regular.ttf` | Liberation fonts (Red Hat / Google) | SIL Open Font License 1.1 — https://scripts.sil.org/OFL |
| `assets/DejaVuSans.ttf` | DejaVu Sans | Bitstream Vera licence with public-domain DejaVu changes — https://dejavu-fonts.github.io/License.html |
| `assets/dict/en_US.aff`, `en_US.dic` | Hunspell US English dictionary from the LibreOffice dictionaries project, derived from SCOWL by Kevin Atkinson (with WordNet-derived inflection data) | SCOWL permissive licence (MIT/BSD-style); full notices in `assets/dict/README_en_US.txt` — https://github.com/LibreOffice/dictionaries |
| `assets/dict/en_GB.aff`, `en_GB.dic` | Hunspell British English dictionary (Aspell-derived, extended by Bartlett, Kelk, Brown and Pinto) from the LibreOffice dictionaries project | LGPL; credits in `assets/dict/README_en_GB.txt` — https://github.com/LibreOffice/dictionaries |
| `assets/icon/typewriter-source.png`, `icon.png`, `icon-1024.png`, `icon.ico` (and the macOS `.icns` made from it in the release build) | App icon: the "content-typewriter" drawing (`streamline-freehand:content-typewriter`) from the Streamline "Freehand free" icon set by Streamline (webalys). Changes: placed on a soft-white rounded square and resized. Found through Iconify | Creative Commons Attribution 4.0 (CC BY 4.0) — https://creativecommons.org/licenses/by/4.0/ · set: https://github.com/webalys-hq/streamline-vectors · https://icon-sets.iconify.design/streamline-freehand/ |

Rust crate dependencies (egui/eframe, krilla, zip, serde, uuid, minisign-verify, …) are used under
their own MIT / Apache-2.0 licences, which are compatible with GPL-3.0.

If you believe a file here is attributed or licensed incorrectly, please
open an issue.
