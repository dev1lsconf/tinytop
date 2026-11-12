import { test, expect } from '@playwright/test';

test.describe('TinyTop dashboard', () => {
  test('renderiza el dashboard, métricas y gráficos sin errores', async ({ page }) => {
    const consoleErrors: string[] = [];
    const pageErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    page.on('pageerror', err => pageErrors.push(err.message));

    await page.goto('/', { waitUntil: 'domcontentloaded' });

    await expect(page).toHaveTitle(/TinyTop/i);
    await expect(page.getByRole('heading', { name: 'TinyTop' })).toBeVisible();
    await expect(page.getByTestId('refresh-btn')).toBeVisible();
    await expect(page.getByTestId('health-badge')).toBeVisible();

    await expect(page.locator('#cpuValue')).not.toHaveText('--%', { timeout: 20_000 });
    await expect(page.locator('#memoryValue')).not.toHaveText('--');
    await expect(page.locator('#diskValue')).not.toHaveText('--');

    await expect.poll(async () => {
      return page.evaluate(() => {
        const cpu = (window as any).Chart?.getChart?.(document.getElementById('cpuChart') as any);
        const mem = (window as any).Chart?.getChart?.(document.getElementById('memoryChart') as any);
        return {
          cpuPoints: cpu?.data?.datasets?.[0]?.data?.length ?? 0,
          memPoints: mem?.data?.datasets?.[0]?.data?.length ?? 0,
          cpuH: (document.getElementById('cpuChart') as HTMLCanvasElement | null)?.clientHeight ?? 0,
        };
      });
    }, { timeout: 30_000 }).toEqual(expect.objectContaining({}));

    const chartState = await page.evaluate(() => {
      const cpu = (window as any).Chart?.getChart?.(document.getElementById('cpuChart') as any);
      const mem = (window as any).Chart?.getChart?.(document.getElementById('memoryChart') as any);
      return {
        cpuPoints: cpu?.data?.datasets?.[0]?.data?.length ?? 0,
        memPoints: mem?.data?.datasets?.[0]?.data?.length ?? 0,
        cpuH: (document.getElementById('cpuChart') as HTMLCanvasElement | null)?.clientHeight ?? 0,
      };
    });
    expect(chartState.cpuPoints).toBeGreaterThan(0);
    expect(chartState.memPoints).toBeGreaterThan(0);
    expect(chartState.cpuH).toBeGreaterThan(0);
    expect(chartState.cpuH).toBeLessThan(500);

    await expect(page.getByTestId('targets-table')).toBeVisible();
    await expect(page.locator('#targetsTable tr')).toHaveCount(2, { timeout: 20_000 });
    await expect(page.locator('#targetsTable')).toContainText(/localhost|api/i);

    const history = await page.evaluate(async () => {
      const r = await fetch('/api/metrics/history?range=24&target=system');
      const d = await r.json();
      return { ok: r.ok, len: Array.isArray(d?.points) ? d.points.length : 0 };
    });
    expect(history.ok).toBeTruthy();
    expect(history.len).toBeGreaterThan(0);

    expect(pageErrors, 'page errors').toEqual([]);
    expect(consoleErrors, 'console errors').toEqual([]);
  });

  test('expone endpoints API principales', async ({ request }) => {
    await expect.poll(async () => (await request.get('/api/health')).status(), { timeout: 30_000 }).toBe(200);

    const health = await request.get('/api/health');
    expect(health.ok()).toBeTruthy();
    const healthJson = await health.json();
    expect(healthJson.version).toBeTruthy();
    expect(healthJson.db?.ok).toBeTruthy();
    expect(healthJson.runtime).toBeTruthy();

    const config = await request.get('/api/config');
    expect(config.ok()).toBeTruthy();
    const cfg = await config.json();
    expect(cfg.version).toBeTruthy();
    expect(Array.isArray(cfg.targets)).toBeTruthy();
    expect(cfg.targets.length).toBeGreaterThan(0);
    expect(typeof cfg.retention_hours).toBe('number');

    const latest = await request.get('/api/metrics/latest');
    expect(latest.ok()).toBeTruthy();
    const data = await latest.json();
    expect(data.system).toBeTruthy();
    expect(Array.isArray(data.targets)).toBeTruthy();
    expect(typeof data.system.cpu_percent).toBe('number');

    const badHistory = await request.get('/api/metrics/history?range=24&target=__missing__');
    expect(badHistory.status()).toBe(404);
  });
});
