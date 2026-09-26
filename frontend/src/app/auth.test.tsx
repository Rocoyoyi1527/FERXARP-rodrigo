import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LoginPage from './(auth)/login/page';
import RegisterPage from './(auth)/register/page';

const push = jest.fn();
const fetchMock = jest.fn<Promise<Response>, [RequestInfo | URL, RequestInit?]>();

jest.mock('next/navigation', () => ({ useRouter: () => ({ push }) }));

function response(status: number, body: unknown = {}): Response {
  return { ok: status < 400, status, headers: new Headers({ 'content-type': 'application/json' }), json: async () => body } as Response;
}

beforeEach(() => {
  localStorage.clear();
  push.mockReset();
  fetchMock.mockReset();
  global.fetch = fetchMock;
});

test('JEST-AUTH-01 login guarda JWT y abre el dashboard', async () => {
  fetchMock.mockResolvedValue(response(200, { token: 'jwt-empresa' }));
  render(<LoginPage />);
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Correo electrónico'), 'empresa@test.mx');
  await user.type(screen.getByLabelText('Contraseña'), 'clave123');
  await user.click(screen.getByRole('button', { name: 'Ingresar' }));
  await waitFor(() => expect(push).toHaveBeenCalledWith('/dashboard'));
  expect(localStorage.getItem('fexarp_token')).toBe('jwt-empresa');
});

test('JEST-AUTH-02 login inválido no guarda token y muestra error seguro', async () => {
  fetchMock.mockResolvedValue(response(401));
  render(<LoginPage />);
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Correo electrónico'), 'empresa@test.mx');
  await user.type(screen.getByLabelText('Contraseña'), 'incorrecta');
  await user.click(screen.getByRole('button', { name: 'Ingresar' }));
  expect(await screen.findByText('Credenciales incorrectas.')).toBeInTheDocument();
  expect(localStorage.getItem('fexarp_token')).toBeNull();
  expect(push).not.toHaveBeenCalled();
});

test('JEST-AUTH-03 registro solo ofrece Empresa y ONG y registra el rol seleccionado', async () => {
  fetchMock.mockResolvedValueOnce(response(200, { message: 'Creada' })).mockResolvedValueOnce(response(200, { token: 'jwt-ong' }));
  render(<RegisterPage />);
  const user = userEvent.setup();
  const options = screen.getAllByRole('option');
  expect(options.map((option) => option.getAttribute('value'))).toEqual(['empresa', 'ong']);
  await user.selectOptions(screen.getByRole('combobox'), 'ong');
  await user.type(screen.getByLabelText('Correo corporativo / institucional'), 'ong@test.mx');
  await user.type(screen.getByLabelText('Contraseña'), 'clave123');
  await user.click(screen.getByRole('button', { name: 'Registrarse' }));
  await waitFor(() => expect(push).toHaveBeenCalledWith('/dashboard'));
  expect(localStorage.getItem('fexarp_token')).toBe('jwt-ong');
  expect(fetchMock.mock.calls[0][1]?.body).toBe(JSON.stringify({ email: 'ong@test.mx', password: 'clave123', role: 'ong' }));
});

test('registro fallido muestra error y evita login posterior', async () => {
  fetchMock.mockResolvedValue(response(409));
  render(<RegisterPage />);
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Correo corporativo / institucional'), 'x@test.mx');
  await user.type(screen.getByLabelText('Contraseña'), 'clave123');
  await user.click(screen.getByRole('button', { name: 'Registrarse' }));
  expect(await screen.findByText(/La operación ya no es válida/)).toBeInTheDocument();
  expect(fetchMock).toHaveBeenCalledTimes(1);
  expect(push).not.toHaveBeenCalled();
});
