#!/usr/bin/env Rscript
suppressPackageStartupMessages({
  library(sf)
  library(jsonlite)
  library(dplyr)
})

output_dir <- "/home/robin/github/bicischools-web/web/public/presets"
dir.create(output_dir, showWarnings = FALSE, recursive = TRUE)

sf_to_geojson <- function(sf_obj) {
  tmp <- tempfile(fileext = ".geojson")
  sf::st_write(sf_obj, tmp, driver = "GeoJSON", quiet = TRUE, delete_dsn = TRUE)
  res <- jsonlite::fromJSON(readLines(tmp, warn = FALSE))
  unlink(tmp)
  res
}

# 1. Lisbon Case Study (Escola Básica Adriano Correia de Oliveira)
cat("Exporting Lisbon case study...\n")
school_lisbon <- readRDS("/home/robin/github/bicischools/paper/Paper_JTG/route-data/school.Rds")
cents_lisbon <- readRDS("/home/robin/github/bicischools/paper/Paper_JTG/route-data/cents_quiet.Rds")
rnet_lisbon <- readRDS("/home/robin/github/bicischools/paper/Paper_JTG/route-data/rnet_quiet.Rds")
routes_lisbon <- readRDS("/home/robin/github/bicischools/paper/Paper_JTG/route-data/to_map_quiet.Rds")

school_coords <- sf::st_coordinates(school_lisbon)[1, 1:2]

lisbon_preset <- list(
  id = "lisbon",
  name = "Lisbon: EB Adriano Correia de Oliveira",
  country = "Portugal",
  city = "Lisbon",
  description = "Case study 1 from the paper: 169 students attending EB Adriano Correia de Oliveira in northern Lisbon.",
  school = list(
    name = "Escola Básica Adriano Correia de Oliveira",
    dgeec_id = 1106908,
    lng = school_coords[1],
    lat = school_coords[2],
    total_students = 169
  ),
  cents = sf_to_geojson(cents_lisbon),
  rnet = sf_to_geojson(rnet_lisbon),
  candidate_routes = sf_to_geojson(routes_lisbon)
)

write(jsonlite::toJSON(lisbon_preset, auto_unbox = TRUE, digits = 6), file.path(output_dir, "lisbon.json"))

# 2. Almada Case Study (Costa da Caparica)
cat("Exporting Almada case study...\n")
cap_cents <- readRDS("/home/robin/github/bicischools/paper/Paper_JTG/route-data/costa-caparica-quiet-centroids.Rds")
cap_routes <- readRDS("/home/robin/github/bicischools/paper/Paper_JTG/route-data/costa-caparica-quiet-routes.Rds")
cap_stops <- sf::read_sf("/home/robin/github/bicischools/paper/Paper_JTG/route-data/costadacaparica.geojson")
cap_csv <- read.csv("/home/robin/github/bicischools/paper/Paper_JTG/route-data/costa-da-caparica.csv")

cap_stops_df <- cap_stops %>%
  mutate(
    lng = sf::st_coordinates(.)[, 1],
    lat = sf::st_coordinates(.)[, 2]
  ) %>%
  sf::st_drop_geometry() %>%
  left_join(cap_csv, by = "Stop")

cap_school_pt <- cap_stops %>% filter(Stop %in% c("Chegada", "End"))
if (nrow(cap_school_pt) == 0) {
  cap_school_coords <- c(-9.233704, 38.6409)
} else {
  cap_school_coords <- sf::st_coordinates(cap_school_pt)[1, 1:2]
}

almada_preset <- list(
  id = "almada",
  name = "Almada: EB nº 2 Costa da Caparica",
  country = "Portugal",
  city = "Almada",
  description = "Case study 2 from the paper: EB nº 2 da Costa da Caparica with real CicloExpresso bike bus schedule and stops.",
  school = list(
    name = "Escola Básica nº 2 da Costa da Caparica",
    dgeec_id = 1503836,
    lng = cap_school_coords[1],
    lat = cap_school_coords[2],
    total_students = 238
  ),
  cents = sf_to_geojson(cap_cents),
  candidate_routes = sf_to_geojson(cap_routes),
  actual_stops = cap_stops_df,
  actual_stops_geojson = sf_to_geojson(cap_stops)
)

write(jsonlite::toJSON(almada_preset, auto_unbox = TRUE, digits = 6), file.path(output_dir, "almada.json"))

# 3. Manchester Case Study (Manley Park Primary School)
if (file.exists("/home/robin/github/bicischools/bicischools/data-raw/routes-manchester.Rds")) {
  cat("Exporting Manchester case study...\n")
  mcr_routes <- readRDS("/home/robin/github/bicischools/bicischools/data-raw/routes-manchester.Rds")
  
  manchester_preset <- list(
    id = "manchester",
    name = "Manchester: Manley Park Primary School",
    country = "UK",
    city = "Manchester",
    description = "Manchester case study: Manley Park Primary School (Whalley Range).",
    school = list(
      name = "Manley Park Primary School",
      lng = -2.25988,
      lat = 53.44905,
      total_students = 430
    ),
    routes = sf_to_geojson(mcr_routes)
  )
  write(jsonlite::toJSON(manchester_preset, auto_unbox = TRUE, digits = 6), file.path(output_dir, "manchester.json"))
}

cat("Preset export complete!\n")
