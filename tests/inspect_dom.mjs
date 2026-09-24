import { chromium } from "playwright";

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  page.on("console", msg => console.log(`[browser.${msg.type()}] ${msg.text()}`));
  page.on("pageerror", e => console.error("[pageerror]", e.message));

  console.log("Navigating to IDE...");
  await page.goto("http://localhost:8765/web-ide/index.html", { waitUntil: "networkidle" });

  console.log("Selecting web_dom_js_demo...");
  await page.selectOption("#examples", "../examples/web_dom_js_demo.rr");
  await page.waitForTimeout(1000);

  console.log("Running...");
  await page.click('#btn-run');
  await page.waitForTimeout(4000);

  const frame = page.frameLocator("#preview");
  
  // Dump DOM elements inside the preview iframe
  const elements = await frame.locator("body *").evaluateAll(els => {
    return els.map(el => {
      const style = window.getComputedStyle(el);
      return {
        id: el.id,
        tagName: el.tagName,
        parent: el.parentElement ? el.parentElement.tagName : null,
        display: style.display,
        position: style.position,
        left: style.left,
        top: style.top,
        width: style.width,
        height: style.height,
        className: el.className,
        innerText: el.innerText ? el.innerText.substring(0, 100) : "",
        html: el.outerHTML.substring(0, 200)
      };
    });
  });

  console.log("DOM Elements in Preview frame:");
  console.log(JSON.stringify(elements, null, 2));

  await browser.close();
})();
