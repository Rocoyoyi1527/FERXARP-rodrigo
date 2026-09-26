import { api, ApiError, apiErrorMessage, isApiError } from './api';

const fetchMock = jest.fn<Promise<Response>, [RequestInfo | URL, RequestInit?]>();

function reply(status: number, body: unknown = {}, contentType = 'application/json'): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: new Headers({ 'content-type': contentType }),
    json: async () => body,
    text: async () => String(body),
  } as Response;
}

beforeEach(() => {
  localStorage.clear();
  fetchMock.mockReset();
  global.fetch = fetchMock;
});

test('JEST-AUTH-01 login envía credenciales y obtiene el token', async () => {
  fetchMock.mockResolvedValue(reply(200, { token: 'jwt-test' }));
  await expect(api.login({ email: 'empresa@test.mx', password: 'clave' })).resolves.toEqual({ token: 'jwt-test' });
  expect(fetchMock).toHaveBeenCalledWith('http://localhost:8000/api/auth/login', expect.objectContaining({
    method: 'POST', body: JSON.stringify({ email: 'empresa@test.mx', password: 'clave' }),
  }));
});

test('JEST-AUTH-02 un 401 de login muestra un error seguro', async () => {
  fetchMock.mockResolvedValue(reply(401));
  await expect(api.login({ email: 'x@y.mx', password: 'mal' })).rejects.toMatchObject({
    status: 401, message: 'Credenciales incorrectas.',
  });
});

test.each([
  [401, 'La sesión ha expirado'],
  [403, 'No tienes permiso'],
  [409, 'La operación ya no es válida'],
])('JEST-API status %i tiene mensaje controlado', async (status, expected) => {
  fetchMock.mockResolvedValue(reply(status));
  await expect(api.getShipments()).rejects.toThrow(expected);
  await expect(api.getShipments()).rejects.toMatchObject({ status });
});

test('adjunta JWT a las peticiones y representa respuestas vacías', async () => {
  localStorage.setItem('fexarp_token', 'jwt-test');
  fetchMock.mockResolvedValue({ ...reply(204), status: 204 });
  await expect(api.approveRequest('donacion-1')).resolves.toEqual({});
  expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining('/approve'), expect.objectContaining({
    headers: expect.objectContaining({ Authorization: 'Bearer jwt-test' }),
  }));
});

test('procesa respuesta de texto y longitud cero', async () => {
  fetchMock.mockResolvedValueOnce({ ...reply(200), headers: new Headers({ 'content-type': 'text/plain' }), text: async () => 'hecho' });
  await expect(api.requestDonation('d1')).resolves.toEqual({ message: 'hecho' });
  fetchMock.mockResolvedValueOnce({ ...reply(200), headers: new Headers({ 'content-length': '0' }) });
  await expect(api.requestDonation('d1')).resolves.toEqual({});
});

test('el payload de rechazo conserva rejection_reason', async () => {
  fetchMock.mockResolvedValue(reply(200, { message: 'Rechazada' }));
  await api.shipmentAction({ donation_id: 'd1', action: 'rechazo', rejection_reason: 'Envase dañado' });
  expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining('/scanner/scan'), expect.objectContaining({
    body: JSON.stringify({ donation_id: 'd1', action: 'rechazo', rejection_reason: 'Envase dañado' }),
  }));
});

test('los endpoints del módulo usan rutas y métodos correctos', async () => {
  fetchMock.mockResolvedValue(reply(200, []));
  await api.register({ email: 'x@y.mx', password: 'clave', role: 'ong' });
  await api.getMe();
  await api.createDonation({ title: 'Arroz', quantity: 2 });
  await api.listDonations();
  await api.getDonationFeed();
  await api.getMatches('d1');
  await api.getMapPoints();
  expect(fetchMock.mock.calls.map(([url]) => String(url))).toEqual(expect.arrayContaining([
    expect.stringContaining('/auth/register'), expect.stringContaining('/auth/me'),
    expect.stringContaining('/donations/feed'), expect.stringContaining('/donations/d1/matches'),
  ]));
});

test('ApiError conserva status y usa fallback seguro para errores desconocidos', () => {
  expect(isApiError(new ApiError(403), 403)).toBe(true);
  expect(isApiError(new Error('x'), 403)).toBe(false);
  expect(apiErrorMessage(new Error('secreto interno'))).toBe('No se pudo conectar con el servidor.');
  expect(new ApiError(418).message).toBe('No se pudo completar la operación.');
});
