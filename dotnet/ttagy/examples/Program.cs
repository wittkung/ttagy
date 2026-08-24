using System;
using Ttagy.Sdk;

Console.WriteLine("⚡ Connecting to TTAgy via .NET 8+ C# SDK...");

var request = new TtagyRequest
{
    SessionId = "session_dotnet_01",
    Prompt = "Demonstrate C# 12 Primary Constructors and Pattern Matching",
    Model = "gemini-3.7-flash",
    Effort = "low",
    TimeoutSecs = 60
};

Console.WriteLine($"Request prepared for session: {request.SessionId} with prompt: {request.Prompt}");
