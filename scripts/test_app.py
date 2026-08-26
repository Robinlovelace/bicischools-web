import asyncio
import os
import sys
from playwright.async_api import async_playwright

ARTIFACT_DIR = "/home/robin/.gemini/antigravity-cli/brain/efeec927-f838-446e-845c-583cfa0be9f8"
os.makedirs(ARTIFACT_DIR, exist_ok=True)

async def run_tests():
    print("Starting Playwright E2E testing for bicischools-web...")
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context(viewport={"width": 1400, "height": 900})
        page = await context.new_page()

        # Capture console messages
        page.on("console", lambda msg: print(f"[Browser Console] {msg.type}: {msg.text}"))
        page.on("pageerror", lambda err: print(f"[Browser Error] {err}"))

        url = "http://localhost:5173/"
        print(f"Navigating to {url}...")
        await page.goto(url, wait_until="networkidle", timeout=15000)

        await page.wait_for_selector("#map canvas", timeout=10000)
        await asyncio.sleep(2)

        # 1. Screenshot Lisbon preset
        print("Taking screenshot of Lisbon case study...")
        lisbon_screenshot = os.path.join(ARTIFACT_DIR, "screenshot_lisbon.png")
        await page.screenshot(path=lisbon_screenshot)

        # 2. Click Almada preset
        print("Testing Almada preset...")
        almada_btn = page.locator("text=Almada: Costa da Caparica")
        await almada_btn.click()
        await asyncio.sleep(2)
        almada_screenshot = os.path.join(ARTIFACT_DIR, "screenshot_almada.png")
        await page.screenshot(path=almada_screenshot)

        # 3. Test Search for Bracken Edge, Leeds
        print("Testing search for 'Bracken Edge, Leeds'...")
        search_input = page.locator('input[placeholder*="Search school"]')
        await search_input.fill("Bracken Edge, Leeds")
        await page.keyboard.press("Enter")
        await asyncio.sleep(2)

        first_result = page.locator("button:has-text('Bracken Edge')").first
        if await first_result.count() > 0:
            print("Found Bracken Edge in search results, clicking...")
            await first_result.click()
        else:
            await page.locator("button:has-text('Run Live Analysis')").click()

        print("Waiting for OSM download and WASM analysis...")
        await asyncio.sleep(6)

        # 4. Adjust Circuity Parameter to 1.5x in Parameters Tab
        print("Testing Circuity parameter adjustment to 1.5x...")
        params_tab = page.locator("button:has-text('Parameters')")
        await params_tab.click()
        await asyncio.sleep(1)

        circuity_slider = page.locator("#circuity")
        await circuity_slider.fill("1.5")
        # Trigger change
        await circuity_slider.dispatch_event("change")
        await asyncio.sleep(4)

        # Take screenshot of circuitous Bracken Edge map
        bracken_screenshot = os.path.join(ARTIFACT_DIR, "screenshot_bracken_edge_circuity.png")
        await page.screenshot(path=bracken_screenshot)
        print(f"Saved: {bracken_screenshot}")

        # 5. Switch to Timetable tab
        print("Switching to Timetable tab...")
        timetable_tab = page.locator("button:has-text('Timetable')")
        await timetable_tab.click()
        await asyncio.sleep(1)

        timetable_screenshot = os.path.join(ARTIFACT_DIR, "screenshot_bracken_edge_timetable.png")
        await page.screenshot(path=timetable_screenshot)
        print(f"Saved: {timetable_screenshot}")

        await browser.close()
        print("All Playwright E2E tests finished successfully!")

if __name__ == "__main__":
    asyncio.run(run_tests())
