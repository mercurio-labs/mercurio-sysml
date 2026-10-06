package Extensions {
    metadata def Tag;
    metadata def Other;
    #Tag def Extended { part nested; }
    #Tag single : Extended { part child; }
    #Tag #Other stacked : Extended;
    #Tag part ordinary;
    part target;
    #Tag connect ordinary to target;
    #Tag connect target to ordinary;
}
