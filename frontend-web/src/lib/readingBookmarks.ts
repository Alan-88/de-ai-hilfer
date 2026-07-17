import type { ReadingContentSegment } from "$lib/readingApi";

export interface ReadingTextTarget {
  segmentIndex: number;
  start: number;
  end: number;
  surface: string;
}

export function insertReadingBookmark(
  body: string,
  segments: ReadingContentSegment[],
  target: ReadingTextTarget,
  headword: string,
) {
  const segment = segments[target.segmentIndex];
  const normalizedHeadword = headword.trim();
  if (!segment || segment.kind !== "text") throw new Error("所选位置已失效");
  if (!normalizedHeadword || /[()\n]/.test(normalizedHeadword)) throw new Error("原型格式无效");

  const surface = segment.text.slice(target.start, target.end);
  if (!surface || surface !== target.surface || /[\[\]\n]/.test(surface)) {
    throw new Error("所选文字已变化，请重新选择");
  }

  const segmentStart = sourceStartForSegment(body, segments, target.segmentIndex);
  const start = segmentStart + target.start;
  const end = segmentStart + target.end;
  return `${body.slice(0, start)}[${surface}](${normalizedHeadword})${body.slice(end)}`;
}

function sourceStartForSegment(body: string, segments: ReadingContentSegment[], targetIndex: number) {
  let cursor = 0;
  for (let index = 0; index < targetIndex; index += 1) {
    const segment = segments[index];
    if (segment.kind === "text") {
      if (body.slice(cursor, cursor + segment.text.length) !== segment.text) throw new Error("文章正文与渲染内容不同步");
      cursor += segment.text.length;
      continue;
    }
    if (body[cursor] !== "[") throw new Error("文章书签位置无效");
    const markerEnd = body.indexOf(")", cursor);
    if (markerEnd < 0) throw new Error("文章书签格式无效");
    cursor = markerEnd + 1;
  }

  const target = segments[targetIndex];
  if (!target || target.kind !== "text" || body.slice(cursor, cursor + target.text.length) !== target.text) {
    throw new Error("文章正文与渲染内容不同步");
  }
  return cursor;
}
