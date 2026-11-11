import { test, expect } from "./fixture";
test.use({ passwordVault: true });
test("create a persistent vault and authenticate its UUID", async ({
  page,
}) => {
  test.setTimeout(120000);
  await page.goto("/");
  await page
    .getByLabel("Master password", { exact: true })
    .fill("correct horse battery staple");
  await page.getByLabel("Confirm master password").fill("different");
  await expect(
    page.getByRole("button", { name: "Create Vault" }),
  ).toBeDisabled();
  await page
    .getByLabel("Confirm master password")
    .fill("correct horse battery staple");
  await page.getByRole("button", { name: "Create Vault" }).click();
  await expect(page.getByRole("button", { name: "Unlock Vault" })).toBeVisible({
    timeout: 30000,
  });
  await expect(page.locator(".auth-footer")).toContainText(
    /UID: [0-9a-f-]{36}/,
  );
  await page.reload();
  await page.getByLabel("Master password", { exact: true }).fill("wrong");
  await page.getByRole("button", { name: "Unlock Vault" }).click();
  await expect(
    page.getByText(
      "The vault could not be opened. Check your password and vault.",
    ),
  ).toBeVisible({ timeout: 30000 });
  await page
    .getByLabel("Master password", { exact: true })
    .fill("correct horse battery staple");
  await page.getByRole("button", { name: "Unlock Vault" }).click();
  await expect(
    page.getByRole("region", { name: "Vault", exact: true }),
  ).toBeVisible({ timeout: 30000 });
});
