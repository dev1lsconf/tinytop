import { test, expect } from '@playwright/test';

test('dashboard loads and charts render', async ({ page }) => {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];
  page.on('console', msg => { if (msg.type() === 'error') consoleErrors.push(msg.text()); });
  page.on('pageerror', err => pageErrors.push(err.message));

  await page.goto('/', { waitUntil: 'networkidle' });
  await page.waitForTimeout(3000);

  await expect(page.locator('h1')).toHaveText('TinyTop');
  const cpuText = await page.locator('#cpuValue').textContent();
  const memText = await page.locator('#memoryValue').textContent();
  console.log('CPU:', cpuText, 'MEM:', memText);

  const cpuChartInfo = await page.evaluate(() => {
    const c = document.getElementById('cpuChart') as HTMLCanvasElement | null;
    const chart = (window as any).Chart?.getChart ? (window as any).Chart.getChart(c) : null;
    // fallback: try Chart.instances in v4 not exposed same; also check dataset
    const anyChart = chart || (c as any)?._chart || null;
    return {
      exists: !!c,
      w: c?.width, h: c?.height, cw: c?.clientWidth, ch: c?.clientHeight,
      found: !!chart,
      labels: chart?.data?.labels?.length ?? null,
      points: chart?.data?.datasets?.[0]?.data?.length ?? null,
    };
  });
  console.log('cpuChartInfo', JSON.stringify(cpuChartInfo));

  const historyStatus = await page.evaluate(async () => {
    const r = await fetch('/api/metrics/history?range=24&target=system');
    const d = await r.json();
    return { ok: r.ok, len: Array.isArray(d) ? d.length : null };
  });
  console.log('history', JSON.stringify(historyStatus));

  await page.screenshot({ path: 'tests/dashboard.png', fullPage: true });

  console.log('consoleErrors', JSON.stringify(consoleErrors));
  console.log('pageErrors', JSON.stringify(pageErrors));

  expect(pageErrors, 'page errors').toEqual([]);
  expect(consoleErrors, 'console errors').toEqual([]);
  expect(cpuText).not.toBe('--%');
});
