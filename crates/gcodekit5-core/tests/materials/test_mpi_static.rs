use gcodekit5_core::data::materials;

#[test]
fn test_mpi_derived_materials_loaded_into_standard_library() {
    let library = materials::init_standard_library();

    for id in [
        "mpi_aisi_304",
        "mpi_aisi_316l",
        "mpi_aisi_4130",
        "mpi_alsi10mg",
        "mpi_al_6061",
        "mpi_d6ac",
        "mpi_in718",
        "mpi_ti_6al_4v_eli",
        "mpi_copper",
    ] {
        assert!(
            library
                .get_material(&materials::MaterialId(id.to_string()))
                .is_some(),
            "missing {id}"
        );
    }
}
