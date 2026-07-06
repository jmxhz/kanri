<!-- SPDX-FileCopyrightText: Copyright (c) 2022-2026 trobonox <hello@trobo.dev> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<template>
  <div
    class="bg-elevation-2 border-elevation-3 focus-within:border-accent mt-1 flex min-h-24 w-full flex-col gap-2 rounded-md border p-2"
    @dragover.prevent
    @drop.prevent="handleDrop"
    @paste="handlePaste"
  >
    <template v-for="(object, index) in document.objects" :key="index">
      <textarea
        v-if="object.type === 'text'"
        v-model="object.text"
        class="text-normal min-h-12 w-full resize-none bg-transparent outline-none"
        maxlength="8000"
        @blur="emitBlur"
        @focus="setFocusedText(index, $event)"
        @input="emitChange"
        @keyup="setFocusedText(index, $event)"
        @mouseup="setFocusedText(index, $event)"
      />
      <button
        v-else
        type="button"
        class="border-elevation-3 bg-elevation-1 bg-elevation-2-hover flex w-fit max-w-full items-center gap-2 rounded-md border p-2 text-left"
        :class="selectedAssetPath === object.asset.blobPath ? 'outline outline-2 outline-accent' : ''"
        @click="selectAsset(index, object.asset.blobPath)"
        @dblclick="openAsset(object.asset.blobPath)"
        @keydown.backspace.prevent="deleteObject(index)"
        @keydown.delete.prevent="deleteObject(index)"
      >
        <img
          v-if="object.asset.kind === 'image'"
          :src="assetUrls[object.asset.blobPath]"
          :class="thumbnailClass"
          class="shrink-0 rounded object-cover"
          draggable="false"
        >
        <div
          v-else
          class="bg-elevation-3 flex size-12 shrink-0 items-center justify-center rounded text-xs font-semibold uppercase"
        >
          {{ getExtension(object.asset.fileName) }}
        </div>
        <span class="max-w-64 break-words text-sm">{{ object.asset.fileName }}</span>
      </button>
    </template>
  </div>
</template>

<script setup lang="ts">
import type { KanriDocument } from "@/utils/documentObjects";

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import {
  createDocument,
  parseDocument,
  serializeDocument,
} from "@/utils/documentObjects";
import { collectTransferFiles, ingestTransferFile } from "@/utils/fileTransfer";

const props = withDefaults(
  defineProps<{
    boardId: string;
    modelValue: string;
    size?: "description" | "task";
  }>(),
  {
    size: "description",
  }
);

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "editorBlurred"): void;
}>();

const document = ref<KanriDocument>(createDocument());
const assetUrls = ref<Record<string, string>>({});
const selectedAssetPath = ref("");
const selectedObjectIndex = ref(-1);
const focusedTextIndex = ref(0);
const focusedTextSelectionStart = ref(0);

const thumbnailClass = computed(() => {
  return props.size === "task" ? "h-[72px] w-[96px]" : "h-[120px] w-[160px]";
});

const refreshAssetUrls = async () => {
  const nextUrls: Record<string, string> = {};
  for (const object of document.value.objects) {
    if (object.type !== "asset" || object.asset.kind !== "image") continue;

    try {
      const filePath = await invoke<string>("kanri_asset_path", {
        blobPath: object.asset.blobPath,
      });
      nextUrls[object.asset.blobPath] = convertFileSrc(filePath);
    } catch (error) {
      console.error("Could not resolve Kanri asset path:", error);
    }
  }
  assetUrls.value = nextUrls;
};

const ensureTextObject = () => {
  if (document.value.objects.length === 0) {
    document.value.objects.push({ type: "text", text: "" });
  }
};

const emitChange = () => {
  emit("update:modelValue", serializeDocument(document.value));
};

const emitBlur = () => {
  emitChange();
  emit("editorBlurred");
};

const setFocusedText = (index: number, event: Event) => {
  const target = event.target;
  if (!(target instanceof HTMLTextAreaElement)) return;

  selectedAssetPath.value = "";
  selectedObjectIndex.value = -1;
  focusedTextIndex.value = index;
  focusedTextSelectionStart.value = target.selectionStart;
};

const selectAsset = (index: number, blobPath: string) => {
  selectedAssetPath.value = blobPath;
  selectedObjectIndex.value = index;
};

const deleteObject = (index: number) => {
  document.value.objects.splice(index, 1);
  selectedAssetPath.value = "";
  selectedObjectIndex.value = -1;
  ensureTextObject();
  emitChange();
};

const insertFilesAtCursor = async (files: File[]) => {
  if (files.length === 0) return;

  const targetIndex =
    document.value.objects[focusedTextIndex.value]?.type === "text"
      ? focusedTextIndex.value
      : document.value.objects.length - 1;
  const target = document.value.objects[targetIndex];
  const text = target?.type === "text" ? target.text : "";
  const selectionStart = Math.min(focusedTextSelectionStart.value, text.length);
  const beforeText = text.slice(0, selectionStart);
  const afterText = text.slice(selectionStart);
  const insertedObjects: KanriDocument["objects"] = [];

  for (const file of files) {
    const asset = await ingestTransferFile(props.boardId, file);
    insertedObjects.push({ type: "asset", asset });
  }

  const replacementObjects: KanriDocument["objects"] = [
    { type: "text", text: beforeText },
    ...insertedObjects,
    { type: "text", text: afterText },
  ];

  document.value.objects.splice(targetIndex, 1, ...replacementObjects);
  focusedTextIndex.value = targetIndex + replacementObjects.length - 1;
  focusedTextSelectionStart.value = 0;
  ensureTextObject();
  emitChange();
  await refreshAssetUrls();
};

const handlePaste = async (event: ClipboardEvent) => {
  const files = collectTransferFiles(event);
  if (files.length === 0) return;

  event.preventDefault();
  const target = event.target;
  if (target instanceof HTMLTextAreaElement) {
    setFocusedText(focusedTextIndex.value, event);
  }
  await insertFilesAtCursor(files);
};

const handleDrop = async (event: DragEvent) => {
  await insertFilesAtCursor(collectTransferFiles(event));
};

const openAsset = async (blobPath: string) => {
  await invoke("kanri_open_asset", { blobPath });
};

const getExtension = (fileName: string) => {
  const extension = fileName.split(".").pop();
  if (!extension || extension === fileName) return "file";
  return extension.slice(0, 4);
};

watch(
  () => props.modelValue,
  async (value) => {
    document.value = parseDocument(value);
    ensureTextObject();
    await refreshAssetUrls();
  },
  { immediate: true }
);
</script>
