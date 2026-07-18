# enter virtual environment
source .venv/bin/activate

# come out of venvironment
# deactivate

#start server
uvicorn src.server:app --reload