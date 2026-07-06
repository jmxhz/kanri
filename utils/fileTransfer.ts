/* SPDX-FileCopyrightText: Copyright (c) 2022-2026 trobonox <hello@trobo.dev>

SPDX-License-Identifier: GPL-3.0-or-later */

import type { KanriAsset } from "@/utils/documentObjects";
import { invoke } from "@tauri-apps/api/core";

type TauriAsset = KanriAsset;

export const ingestTransferFile = async (
  boardId: string,
  file: File
): Promise<KanriAsset> => {
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));

  return await invoke<TauriAsset>("kanri_ingest_bytes", {
    boardId,
    fileName: file.name || "pasted-image.png",
    bytes,
  });
};

export const ingestPathFile = async (
  boardId: string,
  path: string
): Promise<KanriAsset> => {
  return await invoke<TauriAsset>("kanri_ingest_file", {
    boardId,
    path,
  });
};

export const collectTransferFiles = (event: ClipboardEvent | DragEvent) => {
  const transfer = "clipboardData" in event ? event.clipboardData : event.dataTransfer;
  if (!transfer) return [] as File[];

  const files = Array.from(transfer.files || []);
  if (files.length > 0) return files;

  return Array.from(transfer.items || [])
    .filter((item) => item.kind === "file")
    .map((item) => item.getAsFile())
    .filter((file): file is File => file !== null);
};
