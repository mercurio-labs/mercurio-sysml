package SingleType_use_case_redundant { use case def A; use case def B :> A; use case value: A, B; }
