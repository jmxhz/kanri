/* SPDX-FileCopyrightText: Copyright (c) 2026 trobonox <hello@trobo.dev>

SPDX-License-Identifier: GPL-3.0-or-later
*/

function padTwoDigits(value: number) {
  return String(value).padStart(2, "0");
}

export function formatTimestamp(date: Date) {
  const year = date.getFullYear();
  const month = padTwoDigits(date.getMonth() + 1);
  const day = padTwoDigits(date.getDate());
  const hours = padTwoDigits(date.getHours());
  const minutes = padTwoDigits(date.getMinutes());
  const seconds = padTwoDigits(date.getSeconds());

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
}

export function getCurrentTimestamp() {
  return formatTimestamp(new Date());
}
