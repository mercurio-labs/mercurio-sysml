package CrossingMetadata {
    metaclass Mark;
    datatype Value;
    assoc Link {
        end #Mark feature plain : Value[1];
        end cross[0..*] #Mark feature crossed : Value[1];
        end [0..1] #Mark feature anonymous : Value[1];
    }
}
