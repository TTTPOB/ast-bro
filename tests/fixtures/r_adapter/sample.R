#' Normalize a vector.
normalize <- function(values, center = TRUE) {
  if (center) {
    values <- values - mean(values)
  }
  stats::sd(values)
}

DEFAULT_LIMIT <- 100

summarize = \(values) normalize(values)
