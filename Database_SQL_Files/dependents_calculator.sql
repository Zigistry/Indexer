WITH
  base AS (
    SELECT
      lower(
        replace(
          replace(
            replace(rd.url, 'git+https://', ''),
            'https://',
            ''
          ),
          'http://',
          ''
        )
      ) AS url,
      rd.repo_id AS dependent
    FROM
      release_dependencies rd
  ),
  cleaned AS (
    SELECT
      replace(replace(url, 'www.', ''), '.git', '') AS url,
      dependent
    FROM
      base
  ),
  no_query AS (
    SELECT
      /* Remove query strings (?) and fragments (#) */
      CASE
        WHEN instr(url, '?') > 0 THEN substr(url, 1, instr(url, '?') - 1)
        WHEN instr(url, '#') > 0 THEN substr(url, 1, instr(url, '#') - 1)
        ELSE url
      END AS url,
      dependent
    FROM
      cleaned
  ),
  parsed AS (
    SELECT
      CASE
        WHEN url LIKE 'github.com/%' THEN 'gh/'
        WHEN url LIKE 'codeberg.org/%' THEN 'cb/'
      END AS prefix,
      /* Extract everything after github.com/ or codeberg.org/ */
      CASE
        WHEN url LIKE 'github.com/%' THEN substr(url, 12) -- length('github.com/') + 1
        WHEN url LIKE 'codeberg.org/%' THEN substr(url, 14) -- length('codeberg.org/') + 1
      END AS path_after_domain,
      dependent
    FROM
      no_query
    WHERE
      url LIKE 'github.com/%'
      OR url LIKE 'codeberg.org/%'
  ),
  extracted AS (
    SELECT
      /* Split path into parts by '/' and take first two (owner/repo) */
      CASE
        WHEN instr(path_after_domain, '/') > 0 THEN prefix ||
        /* owner name */
        substr(
          path_after_domain,
          1,
          instr(path_after_domain, '/') - 1
        ) || '/' ||
        /* repo name - everything between first and second slash */
        CASE
          WHEN instr(
            substr(
              path_after_domain,
              instr(path_after_domain, '/') + 1
            ),
            '/'
          ) > 0 THEN substr(
            substr(
              path_after_domain,
              instr(path_after_domain, '/') + 1
            ),
            1,
            instr(
              substr(
                path_after_domain,
                instr(path_after_domain, '/') + 1
              ),
              '/'
            ) - 1
          )
          ELSE substr(
            path_after_domain,
            instr(path_after_domain, '/') + 1
          )
        END
      END AS repo_id_of_dependency,
      dependent
    FROM
      parsed
    WHERE
      instr(path_after_domain, '/') > 0 -- Must have at least owner/repo
  )
INSERT OR IGNORE INTO
  repo_dependents (repo_id, dependent_repo_id)
SELECT DISTINCT
  lower(repo_id_of_dependency),
  lower(dependent)
FROM
  extracted
WHERE
  repo_id_of_dependency IS NOT NULL
  AND EXISTS (
    SELECT
      1
    FROM
      repos r
    WHERE
      lower(r.id) = lower(repo_id_of_dependency)
  );

UPDATE repos
SET dependents_count = (
  SELECT count(*)
  FROM repo_dependents
  WHERE repo_dependents.repo_id = repos.id
);
