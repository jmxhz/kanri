/* SPDX-FileCopyrightText: Copyright (c) 2022-2026 trobonox <hello@trobo.dev>

SPDX-License-Identifier: GPL-3.0-or-later */

export type KanriDocumentTextObject = {
  type: "text";
  text: string;
};

export type KanriAssetKind = "image" | "file";

export type KanriAsset = {
  id: string;
  blobPath: string;
  fileName: string;
  kind: KanriAssetKind;
  mimeType?: string | null;
  size?: number | null;
};

export type KanriDocumentAssetObject = {
  type: "asset";
  asset: KanriAsset;
};

export type KanriDocumentObject =
  | KanriDocumentTextObject
  | KanriDocumentAssetObject;

export type KanriDocument = {
  type: "kanri-document";
  version: 1;
  objects: KanriDocumentObject[];
};

const DOCUMENT_TYPE = "kanri-document";

export const createDocument = (
  objects: KanriDocumentObject[] = []
): KanriDocument => ({
  type: DOCUMENT_TYPE,
  version: 1,
  objects,
});

export const parseDocument = (value: string | null | undefined): KanriDocument => {
  if (!value) return createDocument();

  try {
    const parsed = JSON.parse(value) as Partial<KanriDocument>;
    if (
      parsed.type === DOCUMENT_TYPE &&
      parsed.version === 1 &&
      Array.isArray(parsed.objects)
    ) {
      return createDocument(
        parsed.objects.filter((object): object is KanriDocumentObject => {
          if (!object || typeof object !== "object") return false;
          if (object.type === "text") return typeof object.text === "string";
          if (object.type !== "asset") return false;

          return Boolean(object.asset?.blobPath && object.asset?.fileName);
        })
      );
    }
  } catch {
    // Plain strings from older Kanri data remain valid.
  }

  return createDocument([{ type: "text", text: value }]);
};

export const serializeDocument = (document: KanriDocument): string => {
  const hasAsset = document.objects.some((object) => object.type === "asset");
  if (!hasAsset) {
    return document.objects
      .filter((object): object is KanriDocumentTextObject => object.type === "text")
      .map((object) => object.text)
      .join("");
  }

  return JSON.stringify(document);
};

export const isDocumentEmpty = (value: string | null | undefined) => {
  const document = parseDocument(value);
  return document.objects.every((object) => {
    if (object.type === "asset") return false;
    return !/\S/.test(object.text);
  });
};

export const getDocumentAssets = (
  value: string | null | undefined
): KanriAsset[] => {
  return parseDocument(value).objects
    .filter((object): object is KanriDocumentAssetObject => object.type === "asset")
    .map((object) => object.asset);
};

export const replaceDocumentAssetPaths = (
  value: string | null | undefined,
  pathMap: Record<string, string>
) => {
  const document = parseDocument(value);
  let changed = false;

  for (const object of document.objects) {
    if (object.type !== "asset") continue;
    const nextPath = pathMap[object.asset.blobPath];
    if (!nextPath) continue;

    object.asset.blobPath = nextPath;
    changed = true;
  }

  return changed ? serializeDocument(document) : value;
};
