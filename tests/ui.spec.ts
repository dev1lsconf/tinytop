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

    await expect(page.locator('#cpuValue')).not.toHaveText('--%', { timeout: 15_000 });
    await expect(page.locator('#memoryValue')).not.toHaveText('-- MB');
    await expect(page.locator('#diskValue')).not.toHaveText('-- GB');
    await expect(page.locator('#rxValue')).not.toHaveText('--');
    await expect(page.locator('#txValue')).not.toHaveText('--');
    await expect(page.locator('#lastUpdate')).not.toHaveText('—');

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
    }, { timeout: 15_000 }).toMatchObject({
      cpuPoints: expect.any(Number),
      memPoints: expect.any(Number),
      cpuH: expect.any(Number),
    });

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
    expect(chartState.cpuH).toBeLessThan(400);

    await expect(page.getByTestId('targets-table')).toBeVisible();
    await expect(page.locator('#targetsTable tr')).toHaveCount(2, { timeout: 15_000 });
    await expect(page.locator('#targetsTable')).toContainText(/localhost|api/i);

    const history = await page.evaluate(async () => {
      const r = await fetch('/api/metrics/history?range=24&target=system');
      const d = await r.json();
      return { ok: r.ok, len: Array.isArray(d) ? d.length : 0 };
    });
    expect(history.ok).toBeTruthy();
    expect(history.len).toBeGreaterThan(0);

    expect(pageErrors, 'page errors').toEqual([]);
    expect(consoleErrors, 'console errors').toEqual([]);
  });

  test('expone endpoints API principales', async ({ request }) => {
    const health = await request.get('/api/health');
    expect(health.ok()).toBeTruthy();
    await expect(health.json()).resolves.toMatchObject({ status: 'ok' });

    const config = await request.get('/api/config');
    expect(config.ok()).toBeTruthy();
    const cfg = await config.json();
    expect(Array.isArray(cfg.targets)).toBeTruthy();
    expect(cfg.targets.length).toBeGreaterThan(0);

    const latest = await request.get('/api/metrics/latest');
    expect(latest.ok()).toBeTruthy();
    const data = await latest.json();
    expect(Array.isArray(data)).toBeTruthy();
    expect(data.some((x: any) => x.type === 'system')).toBeTruthy();
  });
});
