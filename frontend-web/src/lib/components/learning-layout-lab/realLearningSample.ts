import type { StructuredAnalysisDocument } from "$lib/types";

export const vertragenSample = {
  headword: "vertragen",
  phonetic: "/fərˈtraːɡən/",
  meanings: [
    { part_of_speech: "Verb · stark", chinese: "忍受，容忍；经受住", english: "to tolerate, bear, endure" },
    { part_of_speech: "Verb · stark", chinese: "（与某人）相处；（与某物）相配", english: "to get along with; to go well with" },
    { part_of_speech: "Verb · stark", chinese: "和解，言归于好", english: "to make up (after a quarrel)" },
  ],
  grammar_branches: [{
    selector: "Verb · stark",
    pos: "verb",
    meanings: [
      { zh: "忍受，容忍；经受住", en: "to tolerate, bear, endure" },
      { zh: "（与某人）相处；（与某物）相配", en: "to get along with; to go well with" },
      { zh: "和解，言归于好", en: "to make up (after a quarrel)" },
    ],
    grammar: {
      genders: [], noun_class: "", plural_forms: [], genitive_forms: [],
      separable: "inseparable", transitivity: "both", reflexive: "optional",
      auxiliaries: ["haben"], present_3sg: "verträgt", preterite_3sg: "vertrug",
      partizip_ii: "vertragen", comparative: "", superlative: "",
      governs_cases: ["accusative"], word_order: "",
    },
  }],
  usage_modules: [
    {
      title: "etwas (Akkusativ) vertragen",
      explanation: "表示生理或心理上的“承受能力”或“耐受力”。常用于讨论食物过敏、酒量、气候适应以及对批评或压力的承受度。",
      example_de: "Ich vertrage leider keine Milchprodukte, da ich eine Laktoseintoleranz habe.",
      example_zh: "很遗憾，因为我有乳糖不耐受，所以我不能吃奶制品。",
    },
    {
      title: "sich (gut) mit jemandem vertragen",
      explanation: "表示人与人之间“相处融洽”或“合得来”。强调一种持续的和谐状态，常用于描述兄弟姐妹、邻居或同事之间的关系。",
      example_de: "Die beiden Geschwister haben sich als Kinder oft gestritten, aber heute vertragen sie sich prächtig.",
      example_zh: "这两个亲兄弟小时候经常吵架，但现在他们相处得非常好。",
    },
    {
      title: "etwas vertragen können",
      explanation: "口语常用表达，表示某物“需要”或“如果有了某物会更好”。类似于中文的“这地方要是……就更好了”或“欠缺……”。",
      example_de: "Das Wohnzimmer ist etwas dunkel; es könnte einen helleren Anstrich vertragen.",
      example_zh: "客厅有点暗，如果粉刷得亮堂一点就好了。",
    },
    {
      title: "sich wieder vertragen",
      explanation: "特指在争吵或冲突之后“和解”或“言归于好”。通常暗示双方达成了某种谅解，恢复了和谐关系。",
      example_de: "Nach dem heftigen Wortwechsel haben sie sich zum Glück schnell wieder vertragen.",
      example_zh: "在激烈的争吵之后，幸好他们很快就言归于好了。",
    },
  ],
  deep_insights: [
    {
      title: "语义逻辑：从“携带”到“相容”",
      content_markdown: "核心动词是 `tragen`（背、扛、携带）。前缀 `ver-` 在这里带有一种“处理”或“结果”的色彩。其底层逻辑是：你能否“扛得住”某物（如酒精、批评）而不倒下？或者两个人能否共同“扛起”一段关系而不破裂？这种“承载力”演变成了生理上的“耐受”和社交上的“和睦”。",
    },
    {
      title: "辨析：vertragen vs. ertragen",
      content_markdown: "这两个词都翻译为“忍受”，但侧重点不同。`ertragen` 通常带有“痛苦、沉重”的色彩，强调被动地忍受痛苦、命运或不幸（如 *Schmerzen ertragen* 忍受疼痛）。而 `vertragen` 更多指“兼容性”和“体质”，强调你是否有能力消化或接纳某物（如 *Hitze vertragen* 耐热）。如果你说一个人“verträgt keine Kritik”，是指他性格受不了批评；如果你说他“erträgt die Kritik”，则强调他虽然痛苦但还是忍了下来。",
    },
    {
      title: "语法提醒：及物与反身",
      content_markdown: "当表示“忍受/耐受”某物时，它是及物动词，直接加 Akkusativ（宾格）。当表示“相处”或“和解”时，它必须是反身动词（sich vertragen）。初学者常犯的错误是漏掉反身代词 `sich`，导致句子意思变成“忍受自己”。",
    },
    {
      title: "词源小贴士",
      content_markdown: "法律词汇 `der Vertrag`（合同）其实也源于此。合同的本质就是让原本有分歧的双方“vertragen”（达成一致、和解），从而能够共同“承载”一项协议。",
    },
  ],
  word_network: {
    family: [
      { term: "verträglich", part_of_speech: "adj.", chinese: "易相处的；（食物等）易消化的", english: "compatible; digestible", note: "" },
      { term: "die Verträglichkeit", part_of_speech: "noun, f.", chinese: "兼容性；耐受性", english: "compatibility; tolerance", note: "" },
      { term: "unverträglich", part_of_speech: "adj.", chinese: "不相容的；不能耐受的", english: "incompatible; intolerant", note: "" },
      { term: "der Vertrag", part_of_speech: "noun, m.", chinese: "合同；契约", english: "contract", note: "" },
    ],
    synonyms: [
      { term: "aushalten", part_of_speech: "verb", chinese: "忍受；经受", english: "to endure", note: "" },
      { term: "harmonieren", part_of_speech: "verb", chinese: "和谐；相配", english: "to harmonize", note: "" },
      { term: "sich aussöhnen", part_of_speech: "verb", chinese: "和解", english: "to reconcile", note: "" },
      { term: "tolerieren", part_of_speech: "verb", chinese: "容忍", english: "to tolerate", note: "" },
    ],
    antonyms: [
      { term: "streiten", part_of_speech: "verb", chinese: "争吵", english: "to quarrel", note: "" },
      { term: "sich verkrachen", part_of_speech: "verb", chinese: "闹翻", english: "to fall out with someone", note: "" },
      { term: "ablehnen", part_of_speech: "verb", chinese: "拒绝；排斥", english: "to reject", note: "" },
    ],
  },
  collocations: [], examples: [], grammar_rows: [],
} satisfies StructuredAnalysisDocument;
