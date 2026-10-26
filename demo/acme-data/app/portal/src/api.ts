// Client portal calls into the Clients API.

const API = import.meta.env.VITE_CLIENTS_API_URL;

export async function createClient(input: {
  name: string;
  email: string;
  phone?: string;
  segment: "smb" | "mid" | "enterprise";
}) {
  const res = await fetch(`${API}/clients`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!res.ok) throw new Error(`create client failed: ${res.status}`);
  return (await res.json()) as { client_id: string };
}
