// The manual's "Translations" page (docs/manual/translations.md): the
// translations example (examples/translations/languages.rr) in Chinese and
// in Spanish. Its labels, button and the canvas row at the bottom (the
// greeting drawn with TextOut) show the language's own letters: Chinese
// ideographs from the system's CJK font, Spanish accents from a catalog.
export const topic = "translations";

const LANGUAGES = "examples/translations/languages.rr";

export const scenes = [
  { name: "languages-chinese", program: LANGUAGES, args: ["zh-Hans"], delay: 2 },
  { name: "languages-spanish", program: LANGUAGES, args: ["es"], delay: 2 },
];
