#include <iostream>

const std::string fetch_repo_gql_query_fragment = R"""(
fragment RepoFields on Repository {
  name
  description
  pushedAt
  createdAt
  isArchived
  isDisabled
  isFork
  forkCount
  stargazerCount
  issues {
    totalCount
  }
  watchers {
    totalCount
  }
  defaultBranchRef {
    name
    target {
      ... on Commit {
        oid
      }
    }
  }
  licenseInfo {
    spdxId
  }
  primaryLanguage {
    name
  }
  owner {
    login
    ... on User {
      bio
    }
    ... on Organization {
      description
    }
  }
  repositoryTopics(first: 10) {
    nodes {
      topic {
        name
      }
    }
  }
  releases(first: 5, orderBy: { field: CREATED_AT, direction: DESC }) {
    nodes {
      tagName
      isPrerelease
      publishedAt
    }
  }
  languages(first: 10, orderBy: { field: SIZE, direction: DESC }) {
    nodes {
      name
    }
  }
}
)""";