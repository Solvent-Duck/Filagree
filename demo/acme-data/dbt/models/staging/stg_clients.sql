-- Deduplicate the raw client loads to the latest row per client.

select *
from {{ source('raw', 'clients') }}
qualify row_number() over (partition by client_id order by updated_at desc) = 1
