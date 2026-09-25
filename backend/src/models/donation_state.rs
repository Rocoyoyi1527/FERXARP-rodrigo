#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DonationState {
    EnAcopio,
    Reservado,
    EnTransito,
    Entregado,
    Rechazado,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalAction {
    Entrada,
    Salida,
    Entrega,
    Rechazo,
}

impl DonationState {
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "en_acopio" => Some(Self::EnAcopio),
            "reservado" => Some(Self::Reservado),
            "en_transito" => Some(Self::EnTransito),
            "entregado" => Some(Self::Entregado),
            "rechazado" => Some(Self::Rechazado),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnAcopio => "en_acopio",
            Self::Reservado => "reservado",
            Self::EnTransito => "en_transito",
            Self::Entregado => "entregado",
            Self::Rechazado => "rechazado",
        }
    }

    pub fn can_reserve(self) -> bool {
        self == Self::EnAcopio
    }

    pub fn can_approve(self) -> bool {
        self == Self::Reservado
    }

    pub fn after_physical_action(self, action: PhysicalAction) -> Option<Self> {
        match (self, action) {
            (Self::Reservado, PhysicalAction::Salida) => Some(Self::EnTransito),
            (Self::EnTransito, PhysicalAction::Entrega) => Some(Self::Entregado),
            (Self::EnTransito, PhysicalAction::Rechazo) => Some(Self::Rechazado),
            // Entrada is kept in the wire format for old clients. New donations
            // already start in en_acopio, so it must never rewrite a state.
            _ => None,
        }
    }
}
