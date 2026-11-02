# UI catalogues

Eleven embedded UTF-8 JSON catalogues: `en`, `es`, `de`, `fr`, `it`, `pt-BR`, `pt-PT`, `ru`, `zh-CN`, `ja`, `ko`. Translation keys are the existing Spanish/English UI strings. Each catalogue uses the same key set. Runtime needs no translation service or network access.

Initial translations were assisted by machine translation; animation terminology and principal commands have been reviewed, including frames, bones, weight painting, sketch brushes and pen tablets. Brazilian and European Portuguese have distinct file/save/input-device vocabulary. Further review by native speakers is welcome, especially for longer help text.

Keep named placeholders such as `{name}` and `{heads}` unchanged, including their braces. Preserve intentional prefix/suffix spaces. Numeric units, file-format identifiers and keyboard combinations remain standard. Document names, layer names, symbol names, user text and paths are not translated. Some detailed errors and imported diagnostics retain their source wording; interface labels and static notices use the selected locale.

Update every JSON file when adding a key. `appearance_tests` checks catalogue parity, nonempty translations, preserved placeholders, locale/preferences migration, font coverage, contrast and rendering without changing artwork. `i18n::english` remains the fallback for older strings; new catalogues override it.
