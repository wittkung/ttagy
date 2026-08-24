package com.ttagy.examples;

import com.ttagy.sdk.TtagyRequest;
import com.ttagy.sdk.TtagyResponse;

public class Main {
    public static void main(String[] args) {
        System.out.println("⚡ Connecting to TTAgy via Java 17+ SDK...");

        var request = new TtagyRequest(
            "session_java_01",
            "Explain Java 21 Virtual Threads and Structured Concurrency",
            "gemini-3.7-flash",
            "low",
            null,
            null,
            null,
            60
        );

        System.out.println("Payload created for session: " + request.sessionId());
    }
}
