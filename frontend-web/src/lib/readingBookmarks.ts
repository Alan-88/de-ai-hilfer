import type { ReadingContentSegment } from "$lib/readingApi";

export interface ReadingTextTarget {
  segmentIndex: number;
  start: number;
  end: number;
  surface: string;
}

export interface ReadingTextAnchor {
  offset: number;
  surface: string;
}

export function createReadingTextAnchor(
  segments: ReadingContentSegment[],
  target: ReadingTextTarget,
): ReadingTextAnchor {
  const segment = segments[target.segmentIndex];
  if (!segment || segment.kind !== "text" || segment.text.slice(target.start, target.end) !== target.surface) {
    throw new Error("所选文字已变化，请重新选择");
  }

  let offset = target.start;
  for (let index = 0; index < target.segmentIndex; index += 1) {
    const item = segments[index];
    offset += item.kind === "text" ? item.text.length : item.surface.length;
  }
  return { offset, surface: target.surface };
}

export function resolveReadingTextAnchor(
  segments: ReadingContentSegment[],
  anchor: ReadingTextAnchor,
): ReadingTextTarget {
  let offset = 0;
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    const text = segment.kind === "text" ? segment.text : segment.surface;
    const end = offset + text.length;
    if (anchor.offset >= offset && anchor.offset + anchor.surface.length <= end) {
      if (segment.kind !== "text") throw new Error("所选文字已被标记");
      const start = anchor.offset - offset;
      if (segment.text.slice(start, start + anchor.surface.length) !== anchor.surface) break;
      return { segmentIndex: index, start, end: start + anchor.surface.length, surface: anchor.surface };
    }
    offset = end;
  }
  throw new Error("所选文字已变化，请重新选择");
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

export function removeReadingBookmark(
  body: string,
  segments: ReadingContentSegment[],
  occurrenceIndex: number,
) {
  const targetIndex = segments.findIndex(
    (segment) => segment.kind === "bookmark" && segment.occurrence_index === occurrenceIndex,
  );
  const segment = segments[targetIndex];
  if (!segment || segment.kind !== "bookmark") throw new Error("书签位置已失效");

  const start = sourceStartForSegment(body, segments, targetIndex);
  const surfaceEnd = body.indexOf("](", start + 1);
  const markerEnd = surfaceEnd < 0 ? -1 : body.indexOf(")", surfaceEnd + 2);
  if (body[start] !== "[" || surfaceEnd < 0 || markerEnd < 0 || body.slice(start + 1, surfaceEnd) !== segment.surface) {
    throw new Error("文章书签格式无效");
  }
  return `${body.slice(0, start)}${segment.surface}${body.slice(markerEnd + 1)}`;
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
  if (!target) throw new Error("文章正文与渲染内容不同步");
  if (target.kind === "text" && body.slice(cursor, cursor + target.text.length) !== target.text) {
    throw new Error("文章正文与渲染内容不同步");
  }
  if (target.kind === "bookmark" && body[cursor] !== "[") throw new Error("文章书签位置无效");
  return cursor;
}
