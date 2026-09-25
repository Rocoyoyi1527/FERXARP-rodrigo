"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, DonationItem, ScoredMatch, UserClaims } from "@/lib/api";
import { Navbar } from "@/components/dashboard/Navbar";
import { DonationForm } from "@/components/dashboard/DonationForm";
import { DonationList } from "@/components/dashboard/DonationList";
import { GardenGraph } from "@/components/dashboard/GardenGraph";
import { OngCanopy } from "@/components/dashboard/OngCanopy";
import { CeoMetrics } from "@/components/dashboard/CeoMetrics";
import { AdminAudit } from "@/components/dashboard/AdminAudit";

export default function DashboardPage() {
  const router = useRouter();
  const [user, setUser] = useState<UserClaims | null>(null);
  const [donations, setDonations] = useState<DonationItem[]>([]);
  const [selectedDonation, setSelectedDonation] = useState<DonationItem | null>(null);
  const [matches, setMatches] = useState<ScoredMatch[]>([]);
  const [loadingMatches, setLoadingMatches] = useState(false);
  const [loading, setLoading] = useState(true);

  const loadInitialData = async () => {
    try {
      const claims = await api.getMe();
      setUser(claims);

      if (claims.role === "empresa") {
        const list = await api.listDonations();
        setDonations(list);
      }
    } catch {
      localStorage.removeItem("fexarp_token");
      router.push("/login");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let active = true;
    const token = localStorage.getItem("fexarp_token");
    if (!token) {
      router.push("/login");
    } else {
      api.getMe()
        .then(async (claims) => {
          const list = claims.role === "empresa" ? await api.listDonations() : [];
          if (!active) return;
          setUser(claims);
          setDonations(list);
        })
        .catch(() => {
          if (!active) return;
          localStorage.removeItem("fexarp_token");
          router.push("/login");
        })
        .finally(() => { if (active) setLoading(false); });
    }
    return () => { active = false; };
  }, [router]);

  const handleSelectMatching = async (id: string) => {
    const active = donations.find((d) => d.id === id) || null;
    setSelectedDonation(active);
    setLoadingMatches(true);

    try {
      const results = await api.getMatches(id);
      setMatches(results);
    } catch (err) {
      console.error("Error al calcular matching vectorial:", err);
    } finally {
      setLoadingMatches(false);
    }
  };

  const handleLogout = () => {
    localStorage.removeItem("fexarp_token");
    router.push("/login");
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-garden-obsidian flex items-center justify-center text-xs text-garden-sage font-mono">
        Sincronizando plataforma Fexarp...
      </div>
    );
  }

  return (
    <main className="min-h-screen bg-garden-obsidian text-neutral-100 p-8">
      <Navbar user={user} onLogout={handleLogout} />

      <div className="max-w-6xl mx-auto">
        {user?.role === "ceo" ? (
          <CeoMetrics />
        ) : user?.role === "admin" ? (
          <AdminAudit />
        ) : user?.role === "ong" ? (
          <OngCanopy />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
            <div className="lg:col-span-4 space-y-6">
              <DonationForm onDonationCreated={loadInitialData} />
              <DonationList
                donations={donations}
                onSelectMatching={handleSelectMatching}
              />
            </div>

            <div className="lg:col-span-8">
              <GardenGraph
                donation={selectedDonation}
                matches={matches}
                isLoading={loadingMatches}
              />
            </div>
          </div>
        )}
      </div>
    </main>
  );
}
