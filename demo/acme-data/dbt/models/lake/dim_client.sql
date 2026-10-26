-- One row per client with current contract value: the table analysts query.

select
    c.client_id,
    c.name,
    c.email,
    c.phone,
    c.segment,
    sum(k.arr_cents) / 100.0 as arr_usd,
    c.created_at
from {{ ref('stg_clients') }} c
left join {{ source('raw', 'contracts') }} k
    on k.client_id = c.client_id
   and (k.ends_on is null or k.ends_on >= current_date)
group by all
