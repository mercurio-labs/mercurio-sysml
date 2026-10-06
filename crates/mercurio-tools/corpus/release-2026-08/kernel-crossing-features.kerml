package CrossingFeatures {
    datatype Value;
    assoc Link {
        end crossing[0..*] feature left : Value[1];
        end [0..1] feature right : Value[1];
        end third : Value[1];
    }
}
