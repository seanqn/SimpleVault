import { test, expect } from "./fixture";
import type { Page } from "@playwright/test";
async function unlock(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: "Unlock Vault", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Create your first group" }),
  ).toBeVisible();
}
async function group(page: Page) {
  await page.getByRole("button", { name: "Add group", exact: true }).click();
  await page.getByLabel("Group name", { exact: true }).fill("Personal");
  await page.getByRole("button", { name: "Create group", exact: true }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await page
    .getByLabel("Group", { exact: true })
    .selectOption({ label: "Personal" });
}
async function add(page: Page) {
  await page.getByRole("button", { name: "+ Add", exact: true }).click();
  await page.getByLabel("Organization", { exact: true }).fill("GitHub");
  await page.getByLabel("Username", { exact: true }).fill("alice");
  await page.getByLabel("Password", { exact: true }).fill("fixture-secret");
  await page
    .getByRole("button", { name: "Save credential", exact: true })
    .click();
  await expect(page.locator(".credential-row")).toHaveCount(1);
}

test("legacy styles and real Rust CRUD, search, reveal and relock", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.locator(".auth-card")).toHaveCSS("width", "250px");
  await expect(page.locator(".auth-card")).toHaveCSS("height", "300px");
  await page.screenshot({ path: "test-results/unlock.png" });
  await page.getByRole("button", { name: "Unlock Vault", exact: true }).click();
  await page.setViewportSize({ width: 320, height: 400 });
  await expect(page.locator(".vault-shell")).toHaveCSS(
    "background-color",
    "rgb(43, 43, 43)",
  );
  await expect(page.locator(".vault-shell")).toHaveCSS("border-radius", "12px");
  await group(page);
  await add(page);
  await expect(
    page.getByText("fixture-secret", { exact: true }),
  ).not.toBeVisible();
  await page.screenshot({ path: "test-results/vault.png" });
  await page.getByLabel("Search credentials").fill("nothing");
  await expect(page.getByRole("heading", { name: "No matches" })).toBeVisible();
  await page.getByRole("button", { name: "Clear search" }).click();
  await page.locator(".credential-row").click();
  await page.getByRole("button", { name: "Reveal", exact: true }).click();
  await expect(page.getByLabel("Revealed password")).toHaveText(
    "fixture-secret",
  );
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await expect(page.getByLabel("Revealed password")).not.toBeVisible();
  await page.getByLabel("Organization", { exact: true }).fill("GitHub work");
  await page
    .getByRole("button", { name: "Save credential", exact: true })
    .click();
  await page.getByRole("button", { name: "Lock vault", exact: true }).click();
  await page.getByRole("button", { name: "Unlock Vault", exact: true }).click();
  await page
    .getByLabel("Group", { exact: true })
    .selectOption({ label: "Personal" });
  await page.locator(".credential-row").click();
  await page.getByRole("button", { name: "Reveal", exact: true }).click();
  await expect(page.getByLabel("Revealed password")).toHaveText(
    "fixture-secret",
  );
  await expect(page.getByLabel("Organization", { exact: true })).toHaveValue(
    "GitHub work",
  );
});
test("failed save retains draft; lock cancel, discard and retry", async ({
  page,
  harness,
}) => {
  await unlock(page);
  await group(page);
  await add(page);
  await page.locator(".credential-row").click();
  await page.getByLabel("Organization", { exact: true }).fill("Unsaved");
  harness.failNext("submit_edit");
  await page
    .getByRole("button", { name: "Save credential", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText("draft has been kept");
  await expect(page.getByLabel("Organization", { exact: true })).toHaveValue(
    "Unsaved",
  );
  await page.getByRole("button", { name: "Lock vault", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  await expect(page.getByLabel("Organization", { exact: true })).toHaveValue(
    "Unsaved",
  );
  await page.getByRole("button", { name: "Lock vault", exact: true }).click();
  await page.getByRole("button", { name: "Discard", exact: true }).click();
  await page.getByRole("button", { name: "Unlock Vault", exact: true }).click();
  await page
    .getByLabel("Group", { exact: true })
    .selectOption({ label: "Personal" });
  await page.locator(".credential-row").click();
  await expect(page.getByLabel("Organization", { exact: true })).toHaveValue(
    "GitHub",
  );
  await page.getByLabel("Organization", { exact: true }).fill("Saved at lock");
  await page.getByRole("button", { name: "Lock vault", exact: true }).click();
  harness.failNext("lock");
  await page.getByRole("button", { name: "Save & lock", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("alert")).toContainText("draft has been kept");
  await page.getByRole("button", { name: "Save & lock", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Unlock Vault", exact: true }),
  ).toBeVisible();
});
test("rename and explicit deletion confirmations", async ({ page }) => {
  await unlock(page);
  await group(page);
  await add(page);
  await page.getByRole("button", { name: "Rename", exact: true }).click();
  await page.getByLabel("Group name", { exact: true }).fill("Work");
  await page.getByRole("button", { name: "Save name", exact: true }).click();
  await expect(
    page.getByRole("option", { name: "Work", exact: true }),
  ).toBeAttached();
  await page.locator(".credential-row").click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  await expect(page.getByLabel("Organization", { exact: true })).toHaveValue(
    "GitHub",
  );
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Delete", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Ready when you are" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Delete group", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Delete", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "A little more organized" }),
  ).toBeVisible();
});
test("duplicate-save guard and keyboard/native close handling", async ({
  page,
  harness,
}) => {
  await unlock(page);
  await group(page);
  await page.getByRole("button", { name: "+ Add", exact: true }).click();
  await page.getByLabel("Organization", { exact: true }).fill("Keyboard");
  harness.delayNext("submit_edit", 300);
  const save = page.locator(".editor button[type=submit]");
  await save.click();
  await expect(save).toBeDisabled();
  await expect(page.locator(".credential-row")).toHaveCount(1);
  await page.locator(".credential-row").click();
  await page
    .getByRole("textbox", { name: "Notes", exact: true })
    .fill("keep me");
  await page.evaluate(() => (window as any).testNativeClose());
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Notes", exact: true }),
  ).toHaveValue("keep me");
  await page.evaluate(() => (window as any).testNativeClose());
  await page.getByRole("button", { name: "Save & quit", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Vault closed" }),
  ).toBeVisible();
});

test("a committed change with failed refresh is refreshed without repeating it", async ({
  page,
  harness,
}) => {
  await unlock(page);
  await page.getByRole("button", { name: "Add group", exact: true }).click();
  await page.getByLabel("Group name", { exact: true }).fill("Created once");
  harness.failRefreshAfter("create_group");
  await page.getByRole("button", { name: "Create group", exact: true }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(page.getByRole("alert")).toContainText("The change completed");
  await expect(
    page.getByRole("button", { name: "Add group", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(
    page.getByRole("option", { name: "Created once", exact: true }),
  ).toHaveCount(1);
  await expect(page.getByRole("alert")).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "Add group", exact: true }),
  ).toBeEnabled();
});
