import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ShipmentsPage from './page';
import { api, ApiError, type ShipmentItem, type UserClaims, type UserRole } from '@/lib/api';

const push = jest.fn();
const replace = jest.fn();
jest.mock('next/navigation', () => ({ useRouter: () => ({ push, replace }) }));
jest.mock('@/components/dashboard/Navbar', () => ({ Navbar: () => <nav>Barra</nav> }));

function claims(role: UserRole): UserClaims {
  return { sub: 'usuario', role, exp: 9999999999 };
}

function shipment(status: ShipmentItem['donation_status'], requestStatus: ShipmentItem['request_status'] = 'pendiente'): ShipmentItem {
  return {
    id: 'solicitud-1', donation_id: 'donacion-1', title: 'Arroz', description: null,
    quantity: 4, donation_status: status, request_status: requestStatus,
    donor_email: 'empresa@test.mx', ngo_name: 'ONG Test', assigned_ngo_id: requestStatus === 'aprobada' ? 'ong-1' : null,
    completed_at: status === 'entregado' || status === 'rechazado' ? '2026-09-25T00:00:00Z' : null,
    rejection_reason: status === 'rechazado' ? 'No apto' : null, created_at: null,
  };
}

function mockPage(role: UserRole, items: ShipmentItem[]) {
  jest.spyOn(api, 'getMe').mockResolvedValue(claims(role));
  jest.spyOn(api, 'getShipments').mockResolvedValue(items);
}

beforeEach(() => {
  localStorage.clear();
  localStorage.setItem('fexarp_token', 'jwt-test');
  push.mockReset();
  replace.mockReset();
});
afterEach(() => jest.restoreAllMocks());

test('sin JWT redirige a login antes de pedir datos', async () => {
  localStorage.clear();
  mockPage('empresa', []);
  render(<ShipmentsPage />);
  await waitFor(() => expect(replace).toHaveBeenCalledWith('/login'));
  expect(api.getMe).not.toHaveBeenCalled();
});

test('JEST-API-01 401 limpia la sesión y redirige', async () => {
  jest.spyOn(api, 'getMe').mockRejectedValue(new ApiError(401));
  jest.spyOn(api, 'getShipments').mockResolvedValue([]);
  render(<ShipmentsPage />);
  await waitFor(() => expect(replace).toHaveBeenCalledWith('/login'));
  expect(localStorage.getItem('fexarp_token')).toBeNull();
});

test('JEST-API-02 403 se muestra como falta de permiso', async () => {
  jest.spyOn(api, 'getMe').mockResolvedValue(claims('empresa'));
  jest.spyOn(api, 'getShipments').mockRejectedValue(new ApiError(403));
  render(<ShipmentsPage />);
  expect(await screen.findByRole('status')).toHaveTextContent('No tienes permiso');
});

test('JEST-FLOW-01 reservado pendiente permite aprobar solo a Empresa', async () => {
  mockPage('empresa', [shipment('reservado')]);
  jest.spyOn(api, 'approveRequest').mockResolvedValue();
  render(<ShipmentsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Aprobar solicitud' }));
  await waitFor(() => expect(api.approveRequest).toHaveBeenCalledWith('donacion-1'));
  expect(await screen.findByRole('status')).toHaveTextContent('Solicitud aprobada');
});

test('JEST-API-03 409 al aprobar informa conflicto y actualiza datos', async () => {
  mockPage('empresa', [shipment('reservado')]);
  jest.spyOn(api, 'approveRequest').mockRejectedValue(new ApiError(409));
  render(<ShipmentsPage />);
  const button = await screen.findByRole('button', { name: 'Aprobar solicitud' });
  const before = jest.mocked(api.getShipments).mock.calls.length;
  await userEvent.click(button);
  expect(await screen.findByRole('status')).toHaveTextContent('La operación ya no es válida');
  await waitFor(() => expect(jest.mocked(api.getShipments).mock.calls.length).toBeGreaterThan(before));
});

test.each(['ong', 'ceo', 'admin'] as UserRole[])('reservado pendiente no ofrece Aprobar a %s', async (role) => {
  mockPage(role, [shipment('reservado')]);
  render(<ShipmentsPage />);
  await screen.findByText('Arroz');
  expect(screen.queryByRole('button', { name: 'Aprobar solicitud' })).not.toBeInTheDocument();
});

test.each(['empresa', 'admin'] as UserRole[])('JEST-FLOW-02 reservado aprobado permite salida a %s', async (role) => {
  mockPage(role, [shipment('reservado', 'aprobada')]);
  jest.spyOn(api, 'shipmentAction').mockResolvedValue({ donation_id: 'donacion-1', previous_status: 'reservado', new_status: 'en_transito', message: 'Salida registrada' });
  render(<ShipmentsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Registrar salida' }));
  await waitFor(() => expect(api.shipmentAction).toHaveBeenCalledWith({ donation_id: 'donacion-1', action: 'salida' }));
});

test('reservado aprobado sin ONG asignada no ofrece salida', async () => {
  const item = { ...shipment('reservado', 'aprobada'), assigned_ngo_id: null };
  mockPage('empresa', [item]);
  render(<ShipmentsPage />);
  await screen.findAllByText('Sin registros en esta etapa.');
  expect(screen.queryByRole('button', { name: 'Registrar salida' })).not.toBeInTheDocument();
});

test.each(['ong', 'admin'] as UserRole[])('JEST-FLOW-03 en tránsito permite entregar y rechazar a %s', async (role) => {
  mockPage(role, [shipment('en_transito', 'aprobada')]);
  render(<ShipmentsPage />);
  expect(await screen.findByRole('button', { name: 'Marcar como entregada' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Rechazar entrega' })).toBeInTheDocument();
});

test.each(['empresa', 'ceo'] as UserRole[])('en tránsito no ofrece acciones físicas a %s', async (role) => {
  mockPage(role, [shipment('en_transito', 'aprobada')]);
  render(<ShipmentsPage />);
  await screen.findByText('Arroz');
  expect(screen.queryByRole('button', { name: 'Marcar como entregada' })).not.toBeInTheDocument();
});

test.each(['entregado', 'rechazado'] as ShipmentItem['donation_status'][])('JEST-FLOW-04 %s no ofrece acciones', async (status) => {
  mockPage('ong', [shipment(status, 'aprobada')]);
  render(<ShipmentsPage />);
  await screen.findByText('Arroz');
  expect(screen.queryByRole('button', { name: /Aprobar|Registrar salida|Marcar como entregada|Rechazar entrega/ })).not.toBeInTheDocument();
  if (status === 'rechazado') expect(screen.getByText('Motivo: No apto')).toBeInTheDocument();
});

test('JEST-REJECT-01 motivo vacío no envía request', async () => {
  mockPage('ong', [shipment('en_transito', 'aprobada')]);
  jest.spyOn(api, 'shipmentAction').mockResolvedValue({ donation_id: 'donacion-1', previous_status: 'en_transito', new_status: 'rechazado', message: 'Hecho' });
  render(<ShipmentsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Rechazar entrega' }));
  const dialog = screen.getByRole('dialog');
  expect(dialog).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Confirmar rechazo' })).toBeDisabled();
  const form = screen.getByLabelText('Motivo del rechazo').closest('form');
  expect(form).not.toBeNull();
  fireEvent.submit(form!);
  expect(await screen.findByText('Escribe un motivo de entre 1 y 500 caracteres.')).toBeInTheDocument();
  expect(api.shipmentAction).not.toHaveBeenCalled();
});

test('JEST-REJECT-02 motivo válido se recorta y envía rejection_reason', async () => {
  mockPage('ong', [shipment('en_transito', 'aprobada')]);
  jest.spyOn(api, 'shipmentAction').mockResolvedValue({ donation_id: 'donacion-1', previous_status: 'en_transito', new_status: 'rechazado', message: 'Rechazo registrado' });
  render(<ShipmentsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Rechazar entrega' }));
  await userEvent.type(screen.getByLabelText('Motivo del rechazo'), '  Envase dañado  ');
  await userEvent.click(screen.getByRole('button', { name: 'Confirmar rechazo' }));
  await waitFor(() => expect(api.shipmentAction).toHaveBeenCalledWith({ donation_id: 'donacion-1', action: 'rechazo', rejection_reason: 'Envase dañado' }));
  expect(await screen.findByRole('status')).toHaveTextContent('Rechazo registrado');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

test('el modal de rechazo se cierra con Escape y Cancelar', async () => {
  mockPage('ong', [shipment('en_transito', 'aprobada')]);
  render(<ShipmentsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Rechazar entrega' }));
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole('button', { name: 'Rechazar entrega' }));
  await userEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

test('401 durante entrega invalida la sesión; 409 recarga el estado', async () => {
  mockPage('ong', [shipment('en_transito', 'aprobada')]);
  jest.spyOn(api, 'shipmentAction').mockRejectedValueOnce(new ApiError(409)).mockRejectedValueOnce(new ApiError(401));
  render(<ShipmentsPage />);
  const button = await screen.findByRole('button', { name: 'Marcar como entregada' });
  const before = jest.mocked(api.getShipments).mock.calls.length;
  await userEvent.click(button);
  await waitFor(() => expect(jest.mocked(api.getShipments).mock.calls.length).toBeGreaterThan(before));
  await userEvent.click(screen.getByRole('button', { name: 'Marcar como entregada' }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith('/login'));
  expect(localStorage.getItem('fexarp_token')).toBeNull();
});
