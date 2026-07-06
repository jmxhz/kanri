<!-- SPDX-FileCopyrightText: Copyright (c) 2022-2026 trobonox <hello@trobo.dev> -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

<template>
  <main class="h-screen overflow-auto px-8 py-6">
    <h1 class="text-4xl font-bold">Assets</h1>
    <p class="text-dim-2 mt-1">Board attachment files stored in Kanri data.</p>

    <div class="mt-6 flex flex-col gap-6">
      <section
        v-for="board in assetBoards"
        :key="board.id"
        class="flex flex-col gap-2"
      >
        <h2 class="text-xl font-semibold">{{ board.title }}</h2>
        <div v-if="board.assets.length === 0" class="text-dim-2 text-sm">
          No assets.
        </div>
        <div v-else class="flex flex-col gap-2">
          <div
            v-for="asset in board.assets"
            :key="asset.blobPath"
            class="border-elevation-2 bg-elevation-1 flex items-center justify-between gap-4 rounded-md border p-3"
          >
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <PhFile class="size-5 shrink-0" />
                <span class="break-words font-medium">{{ asset.fileName }}</span>
                <span
                  v-if="!asset.exists"
                  class="rounded bg-red-600 px-1.5 py-0.5 text-xs text-white"
                >
                  Missing
                </span>
                <span
                  v-else-if="!asset.referenced"
                  class="bg-elevation-3 rounded px-1.5 py-0.5 text-xs"
                >
                  Unreferenced
                </span>
              </div>
              <p class="text-dim-2 mt-1 break-all text-xs">{{ asset.blobPath }}</p>
            </div>
            <div class="flex shrink-0 gap-2">
              <button
                class="bg-elevation-2 bg-elevation-3-hover rounded-md px-3 py-1 text-sm"
                :disabled="!asset.exists"
                @click="openAsset(asset.blobPath)"
              >
                Open
              </button>
              <button
                class="bg-elevation-2 bg-elevation-3-hover rounded-md px-3 py-1 text-sm"
                :disabled="!asset.exists"
                @click="revealAsset(asset.blobPath)"
              >
                Locate
              </button>
              <button
                v-if="!asset.referenced"
                class="rounded-md bg-red-600 px-3 py-1 text-sm text-white"
                @click="deleteAsset(asset.blobPath)"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      </section>
    </div>
  </main>
</template>

<script setup lang="ts">
import type { Board } from "@/types/kanban-types";

import { invoke } from "@tauri-apps/api/core";
import { PhFile } from "@phosphor-icons/vue";
import { useTauriStore } from "@/stores/tauriStore";
import { getDocumentAssets } from "@/utils/documentObjects";

type AssetRow = {
  blobPath: string;
  exists: boolean;
  fileName: string;
  referenced: boolean;
};

type BoardAssetRows = {
  id: string;
  title: string;
  assets: AssetRow[];
};

const store = useTauriStore().store;
const assetBoards = ref<BoardAssetRows[]>([]);

const loadAssets = async () => {
  const boards = ((await store.get("boards")) as Board[]) || [];
  const rows: BoardAssetRows[] = [];

  for (const board of boards) {
    const referenced = new Map<string, string>();
    for (const column of board.columns) {
      for (const card of column.cards) {
        for (const asset of getDocumentAssets(card.description)) {
          referenced.set(asset.blobPath, asset.fileName);
        }
        for (const task of card.tasks || []) {
          for (const asset of getDocumentAssets(task.content)) {
            referenced.set(asset.blobPath, asset.fileName);
          }
        }
      }
    }

    const storedPaths = await invoke<string[]>("kanri_list_board_assets", {
      boardId: board.id,
    });
    const allPaths = new Set([...referenced.keys(), ...storedPaths]);
    const assets: AssetRow[] = [];

    for (const blobPath of allPaths) {
      const exists = await invoke<boolean>("kanri_asset_exists", { blobPath });
      assets.push({
        blobPath,
        exists,
        fileName: referenced.get(blobPath) || blobPath.split("/").pop() || "file",
        referenced: referenced.has(blobPath),
      });
    }

    rows.push({
      id: board.id,
      title: board.title,
      assets: assets.sort((a, b) => a.fileName.localeCompare(b.fileName)),
    });
  }

  assetBoards.value = rows;
};

const openAsset = async (blobPath: string) => {
  await invoke("kanri_open_asset", { blobPath });
};

const revealAsset = async (blobPath: string) => {
  await invoke("kanri_reveal_asset", { blobPath });
};

const deleteAsset = async (blobPath: string) => {
  await invoke("kanri_delete_asset", { blobPath });
  await loadAssets();
};

onMounted(loadAssets);
</script>
