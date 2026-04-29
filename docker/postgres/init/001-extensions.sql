CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;

LOAD 'age';

GRANT USAGE ON SCHEMA ag_catalog TO primerlab;

SET search_path = ag_catalog, "$user", public;

SELECT ag_catalog.create_graph('primer_memory')
WHERE NOT EXISTS (
    SELECT 1
    FROM ag_catalog.ag_graph
    WHERE name = 'primer_memory'
);
