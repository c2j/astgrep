// @rule GAUSSDB-SET-001
// @desc Java DAO with UPDATE SET 3+ columns from subquery
// @expect MATCH
package com.example.dao;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.SQLException;

public class EmployeeDao {

    public void updateEmployees(Connection conn, int targetId) throws SQLException {
        String sql = "UPDATE employees SET (salary, dept, title) = "
                + "(SELECT salary, dept, title FROM new_data WHERE id = ?)";
        PreparedStatement ps = conn.prepareStatement(sql);
        ps.setInt(1, targetId);
        ps.executeUpdate();
    }
}
