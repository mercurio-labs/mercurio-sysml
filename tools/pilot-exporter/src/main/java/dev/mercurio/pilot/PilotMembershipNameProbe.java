package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.lang.sysml.SysMLFactory;

/** Independent stored Membership attribute controls; no transformation shortcuts. */
public final class PilotMembershipNameProbe {
    public static void main(String[] args) throws Exception {
        var rows = new ArrayList<Object>();
        for (int mode = 0; mode < 6; mode++) {
            var member = SysMLFactory.eINSTANCE.createMembership();
            if (mode == 1) member.setMemberName("Long");
            if (mode == 2) member.setMemberShortName("Short");
            if (mode == 3) { member.setMemberName("Long"); member.setMemberShortName("Short"); }
            if (mode == 4) member.setMemberName("");
            if (mode == 5) { member.setMemberName("Long"); member.setMemberName(null); }
            var row = new TreeMap<String,Object>();
            row.put("mode", mode);
            row.put("member_name", member.getMemberName());
            row.put("member_short_name", member.getMemberShortName());
            rows.add(row);
        }
        Files.writeString(Path.of(args[0]), new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(rows));
    }
}
