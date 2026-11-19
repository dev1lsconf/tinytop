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
    await expect(page.locator('#netRateValue')).toBeVisible();

    // UI layout: stats cards should be evenly aligned (same height/width)
    const statsLayout = await page.evaluate(() => {
      const cards = Array.from(document.querySelectorAll('[data-testid="stats-grid"] .card')) as HTMLElement[];
      if (cards.length !== 3) return { count: cards.length, heights: [], widths: [], tops: [] };
      const rects = cards.map(c => c.getBoundingClientRect());
      return {
        count: cards.length,
        heights: rects.map(r => Math.round(r.height)),
        widths: rects.map(r => Math.round(r.width)),
        tops: rects.map(r => Math.round(r.top)),
      };
    });
    expect(statsLayout.count).toBe(3);
    expect(new Set(statsLayout.tops).size).toBe(1);
    expect(new Set(statsLayout.heights).size).toBe(1);
    expect(new Set(statsLayout.widths).size).toBe(1);
    await expect(page.locator('[data-testid="card-disk"]')).toHaveCount(0);

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
    await expect(page.locator('#targetsTable tr')).toHaveCount(3, { timeout: 20_000 });
    await expect(page.locator('#targetsTable')).toContainText(/localhost|api|freellmapi/i);

    // extended sections
    await expect(page.getByTestId('alerts-card')).toBeVisible();
    await expect(page.getByTestId('events-card')).toBeVisible();
    await expect(page.locator('#alertsTable')).toBeVisible();
    await expect(page.locator('#eventsTable')).toBeVisible();
    await expect(page.getByTestId('export-json')).toBeVisible();
    await expect(page.getByTestId('export-csv')).toBeVisible();
    await expect(page.locator('#kvLoad')).toBeVisible();
    await expect(page.locator('#kvUptime')).toBeVisible();

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

  test('muestra panel de token y permite guardar/borrar sin romper la UI', async ({ page }) => {
    await page.goto('/', { waitUntil: 'domcontentloaded' });

    const panel = page.getByTestId('auth-panel');
    await expect(panel).toHaveClass(/hidden/);

    await page.getByTestId('auth-btn').click();
    await expect(panel).not.toHaveClass(/hidden/);
    await expect(page.locator('#authTokenInput')).toBeVisible();
    await expect(page.locator('#authSaveBtn')).toBeVisible();
    await expect(page.locator('#authClearBtn')).toBeVisible();

    await page.locator('#authTokenInput').fill('test-token');
    await page.locator('#authSaveBtn').click();
    await expect.poll(async () => page.evaluate(() => sessionStorage.getItem('tinytop_token'))).toBe('test-token');
    await expect(page.locator('#authPanelStatus')).toContainText(/Token configurado/);

    await page.locator('#authClearBtn').click();
    await expect.poll(async () => page.evaluate(() => sessionStorage.getItem('tinytop_token'))).toBeNull();

    await page.locator('#authCancelBtn').click();
    await expect(panel).toHaveClass(/hidden/);
  });

  test('muestra panel de token cuando la API responde 401 y reintenta con Authorization', async ({ page }) => {
    await page.addInitScript(() => sessionStorage.removeItem('tinytop_token'));
    let protectedSeen = false;
    await page.route('**/api/metrics/latest**', async route => {
      const auth = route.request().headers()['authorization'];
      if (auth === 'Bearer secret') {
        protectedSeen = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            system: {
              cpu_percent: 1, mem_used_percent: 2, disk_used_percent: 3,
              mem_used_mb: 10, mem_total_mb: 100, disk_used_mb: 10, disk_total_mb: 100,
              net_rx_bytes: 0, net_tx_bytes: 0, net_rx_bytes_per_sec: 0, net_tx_bytes_per_sec: 0,
              load1: 0.1, load5: 0.1, load15: 0.1, uptime_seconds: 100, top_processes: '[]'
            },
            targets: []
          }),
        });
      } else {
        await route.fulfill({ status: 401, contentType: 'application/json', body: JSON.stringify({ error: { code: 'unauthorized', message: 'authentication required' } }) });
      }
    });
    await page.route('**/api/metrics/history**', route => route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }));
    await page.route('**/api/alerts**', route => route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }));
    await page.route('**/api/events**', route => route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }));

    await page.goto('/', { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('auth-panel')).not.toHaveClass(/hidden/, { timeout: 20_000 });
    await expect(page.getByTestId('error-banner')).toContainText(/No autorizado|autentic/i, { timeout: 20_000 });

    await page.locator('#authTokenInput').fill('secret');
    await page.locator('#authSaveBtn').click();

    await expect.poll(() => protectedSeen, { timeout: 20_000 }).toBeTruthy();
  });

  test('expone endpoints API principales y nuevos', async ({ request }) => {
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

    const alerts = await request.get('/api/alerts?limit=10');
    expect(alerts.ok()).toBeTruthy();
    const alertsJson = await alerts.json();
    expect(Array.isArray(alertsJson.alerts)).toBeTruthy();

    const events = await request.get('/api/events?limit=10');
    expect(events.ok()).toBeTruthy();
    const eventsJson = await events.json();
    expect(Array.isArray(eventsJson.events)).toBeTruthy();

    const expJson = await request.get('/api/export?target=system&format=json&range=24');
    expect(expJson.ok()).toBeTruthy();
    const expCsv = await request.get('/api/export?target=system&format=csv&range=24');
    expect(expCsv.ok()).toBeTruthy();
    expect(await expCsv.text()).toContain('timestamp');

    const prom = await request.get('/metrics');
    expect(prom.ok()).toBeTruthy();
    expect(await prom.text()).toContain('tinytop_up');

    const badHistory = await request.get('/api/metrics/history?range=24&target=__missing__');
    expect(badHistory.status()).toBe(404);
  });
});
