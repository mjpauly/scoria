-- Create Feedback Table
CREATE TABLE feedback(
   id uuid NOT NULL,
   PRIMARY KEY (id),
   subject TEXT NOT NULL,
   body TEXT NOT NULL,
   email TEXT NULL,
   source TEXT NULL,
   sent_at timestamptz NOT NULL
);
