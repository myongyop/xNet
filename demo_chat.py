import requests
import json
import sys

API_URL = "http://localhost:3030/api/v1/task"

def chat_with_xnet():
    print("=== xNet Chat Demo (Service API) ===")
    print(f"Connecting to Local Node at {API_URL}...")
    
    while True:
        try:
            prompt = input("\nYou: ")
            if prompt.lower() in ['exit', 'quit']:
                break
            
            payload = {
                "model": "tinyllama",
                "prompt": prompt
            }
            
            print("Sending task to xNet...")
            response = requests.post(API_URL, json=payload)
            
            if response.status_code == 200:
                data = response.json()
                if data.get("status") == "completed":
                    result = data.get("result", "No result")
                    print(f"\nxNet AI: {result}\n")
                else:
                    print(f"xNet: Task Queued! (ID: {data.get('task_id', 'unknown')})")
                    print("(Note: Task sent to network)")
            else:
                print(f"Error: {response.status_code} - {response.text}")
                
        except requests.exceptions.ConnectionError:
            print("Error: Could not connect to xNet Node. Is the Desktop App running?")
            break
        except Exception as e:
            print(f"An error occurred: {e}")
            break

if __name__ == "__main__":
    chat_with_xnet()
