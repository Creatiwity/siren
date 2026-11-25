// Load test plan for the Siren API (k6).
//
//   k6 run loadtest/k6/siren.js                                  # smoke, localhost
//   BASE_URL=https://staging.example PROFILE=load k6 run loadtest/k6/siren.js
//   API_SECRET=... PROFILE=stress RATE=0.5 k6 run loadtest/k6/siren.js
//
// Environment:
//   BASE_URL    target (default http://localhost:8080)
//   API_SECRET  sent as x-client-secret when set (gateway ext-authz)
//   PROFILE     smoke | load | stress | spike | soak (default smoke)
//   RATE        multiplier applied to every arrival rate (default 1)
//   GEOCODING   false to skip the address scenarios (no index loaded)
//   GEO_SORT    sort sent by the geo scenario (e.g. distance); default: none
//   ONLY        comma-separated scenarios to run (e.g. search_geo,search_text)
//   DURATION    duration of the load profile (default 10m)
//
// The traffic mix follows production (Sentry, November 2026): direct lookups
// are ~99% of requests, 57% by SIREN and 43% by SIRET. Searches and geocoding
// are rare today but expensive, so they get their own scenarios and
// thresholds instead of being drowned in the lookups.
//
// Through the gateway, the API key's tier rate limit applies: 429 responses
// are counted in `rate_limited`, not as errors, and mean the test measured the
// limiter rather than the API. Use a key with a high tier, or target the
// service directly.

import http from 'k6/http';
import { check, fail } from 'k6';
import { Counter } from 'k6/metrics';

const BASE_URL = (__ENV.BASE_URL || 'http://localhost:8080').replace(/\/$/, '');
const PROFILE = __ENV.PROFILE || 'smoke';
const RATE = Number(__ENV.RATE || 1);
const GEOCODING = __ENV.GEOCODING !== 'false';
// Explicit sort for the geo scenario; empty keeps the server default.
const GEO_SORT = __ENV.GEO_SORT || '';

const params = (name) => ({
  headers: __ENV.API_SECRET ? { 'x-client-secret': __ENV.API_SECRET } : {},
  tags: { name },
  // Above the 15 s gateway timeout: a 504 is reported as such, not as a k6 timeout.
  timeout: '20s',
});

const rateLimited = new Counter('rate_limited');

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

// City centres, metropolitan and overseas, used to collect real identifiers
// and as geo search centres.
const CENTRES = [
  [48.8566, 2.3522], [45.764, 4.8357], [43.2965, 5.3698], [43.6047, 1.4442],
  [43.7102, 7.262], [47.2184, -1.5536], [48.5734, 7.7521], [43.6108, 3.8767],
  [44.8378, -0.5792], [50.6292, 3.0573], [48.1173, -1.6778], [49.2583, 4.0317],
  [45.1885, 5.7245], [47.322, 5.0415], [49.4431, 1.0993], [45.7772, 3.087],
  [-20.8789, 55.4481], [14.6161, -61.0588], [16.2411, -61.5331], [4.9372, -52.326],
];

// Complete addresses: what /v3/adresses/search and `address=` expect.
const ADDRESSES = [
  '10 avenue des champs elysees paris',
  '1 rue de rivoli paris',
  '20 place bellecour lyon',
  '2 quai du port marseille',
  '1 place du capitole toulouse',
  '5 rue sainte catherine bordeaux',
  '3 place kleber strasbourg',
  '10 rue nationale lille',
  '1 promenade des anglais nice',
  '8 rue de la paix nantes',
  '12 rue de la republique rennes',
  '4 place de la comedie montpellier',
];

// What a user has typed so far: the last word is incomplete.
const PREFIXES = [
  '10 avenue des champs ely', '1 rue de riv', '20 place bellec', '2 quai du po',
  '1 place du capit', '5 rue sainte cath', '3 place kleb', '10 rue nation',
  '8 rue de la pa', '12 rue de la repub', '4 place de la com', '1 promenade des ang',
];

// Text searches, including typos that go through lexicon correction and
// words rare enough to reach the trigram fallback.
const QUERIES = [
  'boulangerie', 'pharmacie', 'garage', 'restaurant', 'coiffure', 'boucherie',
  'carrefour', 'carefour', 'leclerc', 'boulengerie', 'pharmacei', 'creatiwity',
  'renault', 'credit agricole', 'la poste', 'mairie',
];
const COMMUNES = ['paris', 'lyon', 'marseille', 'toulouse', 'nantes', 'lille', 'saint etienne'];

const pick = (list) => list[Math.floor(Math.random() * list.length)];
const jitter = (value, spread) => value + (Math.random() - 0.5) * spread;

// ---------------------------------------------------------------------------
// Profiles (arrival rates in requests per second, before RATE)
// ---------------------------------------------------------------------------

// Production today averages ~3.4 lookups/s. `load` is about 5x that.
const BASE_RATES = {
  lookup_siren: 11,
  lookup_siret: 9,
  search_text: 2,
  search_geo: 1,
  search_address: 1,
  adresses_autocomplete: 5,
  adresses_search: 2,
};

const EXEC = {
  lookup_siren: 'lookupSiren',
  lookup_siret: 'lookupSiret',
  search_text: 'searchText',
  search_geo: 'searchGeo',
  search_address: 'searchAddress',
  adresses_autocomplete: 'adressesAutocomplete',
  adresses_search: 'adressesSearch',
};

function rate(name, factor = 1) {
  return Math.max(1, Math.round(BASE_RATES[name] * RATE * factor));
}

function constant(name, duration, factor = 1) {
  const r = rate(name, factor);
  return {
    executor: 'constant-arrival-rate',
    exec: EXEC[name],
    rate: r,
    timeUnit: '1s',
    duration,
    preAllocatedVUs: Math.max(2, r * 2),
    maxVUs: Math.max(10, r * 20),
  };
}

function ramping(name, stages) {
  const top = Math.max(...stages.map((s) => s[1]));
  return {
    executor: 'ramping-arrival-rate',
    exec: EXEC[name],
    startRate: rate(name, stages[0][1]),
    timeUnit: '1s',
    stages: stages.map(([duration, factor]) => ({ duration, target: rate(name, factor) })),
    preAllocatedVUs: Math.max(2, rate(name, top) * 2),
    maxVUs: Math.max(10, rate(name, top) * 20),
  };
}

function scenariosFor(profile) {
  const only = (__ENV.ONLY || '').split(',').map((s) => s.trim()).filter(Boolean);
  const names = Object.keys(BASE_RATES).filter(
    (name) => (GEOCODING || !['search_address', 'adresses_autocomplete', 'adresses_search'].includes(name))
      && (only.length === 0 || only.includes(name)),
  );
  const build = (make) => Object.fromEntries(names.map((name) => [name, make(name)]));

  switch (profile) {
    case 'smoke':
      // Every endpoint, one request per second, to validate the script and the target.
      return build((name) => ({ ...constant(name, '1m'), rate: 1, preAllocatedVUs: 1, maxVUs: 3 }));
    case 'load':
      // Steady load at the base rates.
      return build((name) => constant(name, __ENV.DURATION || '10m'));
    case 'stress':
      // Up to 5x the base rates in steps, to find where latency breaks.
      return build((name) => ramping(name, [
        ['2m', 1], ['3m', 1], ['2m', 2], ['3m', 2], ['2m', 3], ['3m', 3], ['2m', 5], ['3m', 5], ['2m', 0],
      ]));
    case 'spike':
      // Lookups jump to 10x for a minute, searches stay at base rate.
      return build((name) => (name.startsWith('lookup_')
        ? ramping(name, [['1m', 1], ['10s', 10], ['1m', 10], ['10s', 1], ['2m', 1]])
        : constant(name, '4m30s')));
    case 'soak':
      // Base rates for an hour: pool exhaustion, memory, cache churn.
      return build((name) => constant(name, '1h'));
    default:
      fail(`unknown PROFILE ${profile}`);
  }
}

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

// Server-side p95 before this test: lookups ~8 ms, text search ~150 ms,
// geocoding ~10 ms. The budgets below include the network to the target.
export const options = {
  scenarios: scenariosFor(PROFILE),
  thresholds: {
    http_req_failed: ['rate<0.01'],
    checks: ['rate>0.99'],
    'http_req_duration{name:GET /v3/unites_legales/{siren}}': ['p(95)<150', 'p(99)<500'],
    'http_req_duration{name:GET /v3/etablissements/{siret}}': ['p(95)<150', 'p(99)<500'],
    'http_req_duration{name:GET /v3/etablissements?q}': ['p(95)<1000'],
    'http_req_duration{name:GET /v3/etablissements?lat&lng&radius}': ['p(95)<1500'],
    'http_req_duration{name:GET /v3/adresses/autocomplete}': ['p(95)<300'],
    'http_req_duration{name:GET /v3/adresses/search}': ['p(95)<300'],
    'http_req_duration{name:GET /v3/etablissements?address}': ['p(95)<1500'],
  },
  summaryTrendStats: ['avg', 'med', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

// ---------------------------------------------------------------------------
// Setup: real identifiers from the target itself
// ---------------------------------------------------------------------------

export function setup() {
  const sirets = new Set();
  const sirens = new Set();
  for (const [lat, lng] of CENTRES) {
    const res = http.get(
      `${BASE_URL}/v3/etablissements?lat=${lat}&lng=${lng}&radius=1000&sort=distance&limit=100`,
      params('setup'),
    );
    if (res.status !== 200) {
      fail(`setup: ${res.status} on ${res.url} (BASE_URL, API_SECRET?)`);
    }
    for (const e of res.json('etablissements') || []) {
      sirets.add(e.siret);
      sirens.add(e.siren);
    }
  }
  if (sirets.size === 0) {
    fail('setup: no establishment found around the city centres');
  }
  console.log(`setup: ${sirets.size} SIRET, ${sirens.size} SIREN, profile ${PROFILE}, rate x${RATE}`);
  return { sirets: [...sirets], sirens: [...sirens] };
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

function expect(res, name, ok = [200]) {
  if (res.status === 429) {
    rateLimited.add(1, { name });
    return;
  }
  check(res, { [`${name}: ${ok.join('/')}`]: (r) => ok.includes(r.status) });
}

export function lookupSiren(data) {
  const name = 'GET /v3/unites_legales/{siren}';
  // 301: a duplicate SIREN redirects to its canonical one.
  expect(http.get(`${BASE_URL}/v3/unites_legales/${pick(data.sirens)}`, params(name)), name, [200, 301]);
}

export function lookupSiret(data) {
  const name = 'GET /v3/etablissements/{siret}';
  expect(http.get(`${BASE_URL}/v3/etablissements/${pick(data.sirets)}`, params(name)), name, [200, 301]);
}

export function searchText() {
  const name = 'GET /v3/etablissements?q';
  const q = encodeURIComponent(pick(QUERIES));
  // A third with a commune, which takes the composite FTS + commune index.
  const commune = Math.random() < 1 / 3 ? `&commune=${encodeURIComponent(pick(COMMUNES))}` : '';
  expect(http.get(`${BASE_URL}/v3/etablissements?q=${q}${commune}`, params(name)), name);
}

export function searchGeo() {
  const name = 'GET /v3/etablissements?lat&lng&radius';
  const [lat, lng] = pick(CENTRES);
  // Jittered by ~2 km so that the test does not only hit warm pages; radius
  // from 100 m to 2 km (the 2 km one in a dense centre is the expensive case).
  const radius = pick([100, 300, 500, 1000, 2000]);
  const sort = GEO_SORT ? `&sort=${GEO_SORT}` : '';
  const url = `${BASE_URL}/v3/etablissements?lat=${jitter(lat, 0.04).toFixed(5)}&lng=${jitter(lng, 0.04).toFixed(5)}&radius=${radius}${sort}`;
  const p = params(name);
  p.tags.radius = String(radius);
  expect(http.get(url, p), name);
}

export function searchAddress() {
  const name = 'GET /v3/etablissements?address';
  const address = encodeURIComponent(pick(ADDRESSES));
  const q = Math.random() < 0.5 ? `&q=${encodeURIComponent(pick(QUERIES))}` : '';
  expect(http.get(`${BASE_URL}/v3/etablissements?address=${address}${q}`, params(name)), name);
}

export function adressesAutocomplete() {
  const name = 'GET /v3/adresses/autocomplete';
  // Type-ahead: successive prefixes of one address, like a user typing.
  // The API trims q and wants at least 3 characters: never cut right after a
  // space that would leave fewer.
  const full = pick(PREFIXES);
  let cut = 3 + Math.floor(Math.random() * (full.length - 2));
  while (full.slice(0, cut).trim().length < 3) cut += 1;
  const q = encodeURIComponent(full.slice(0, cut).trim());
  expect(http.get(`${BASE_URL}/v3/adresses/autocomplete?q=${q}&limit=5`, params(name)), name);
}

export function adressesSearch() {
  const name = 'GET /v3/adresses/search';
  const q = encodeURIComponent(pick(ADDRESSES));
  expect(http.get(`${BASE_URL}/v3/adresses/search?q=${q}&limit=1`, params(name)), name);
}
