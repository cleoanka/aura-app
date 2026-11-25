import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
  Channel: class {},
}));

import { pickVaultFolder } from "./ipc";

describe("pickVaultFolder", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue("/repo-b");
  });

  it("varsayılan olarak aktif workspace'i değiştirmez (activate=false)", async () => {
    await expect(pickVaultFolder()).resolves.toBe("/repo-b");
    expect(invoke).toHaveBeenCalledWith("pick_vault_folder", { activate: false });
  });

  it("yalnız açıkça istenirse workspace'i değiştirir", async () => {
    await pickVaultFolder(true);
    expect(invoke).toHaveBeenCalledWith("pick_vault_folder", { activate: true });
  });
});
